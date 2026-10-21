use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use serde_json::{Value, json};
use tinybus::{
    BusName, Connection,
    module::{ModuleHost, ModuleInfo},
};
use tinystoragedrivers_bus::{CollectionSpec, StorageNeeds};
use tinystoragedrivers_busproxy::{
    AuditEvent, AuditSink, Environment, GovernedEndpointLimits, GovernedStorage, GrantPolicy,
    HostContext, HostFailure, HostStorageContext, ResolvedStorage, StorageDescriptor, StorageGrant,
    StorageLease,
};
use tinystoragedrivers_client::StorageServer;
use tinystoragedrivers_core::{
    MemoryStorage, Precondition, Scope, StorageBackend, WriteOptions,
    schema::delivery::MigrationDeliveryClient,
    schema::{MigrateOptions, Registry},
};
use tinystoragedrivers_tinybus::{
    StorageRegistration, attach_storage,
    migration::{
        EXCHANGE_MEMBER, GET_SCHEMA_MEMBER, MIGRATION_INTERFACE, MIGRATION_OBJECT_PATH,
        MigrationModuleClient,
    },
};

type TestResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone)]
struct Profile(&'static str);
impl HostContext for Profile {}

struct StorageEnvironment {
    backend: Arc<MemoryStorage>,
    collections: Vec<CollectionSpec>,
}

#[async_trait]
impl Environment<Profile> for StorageEnvironment {
    async fn resolve(
        &self,
        context: &Profile,
        _module: &tinybus::module::ModuleId,
    ) -> Result<ResolvedStorage, HostFailure> {
        let scope = Scope::new(context.0).map_err(|_| HostFailure::Denied("invalid-scope"))?;
        let storage = self
            .backend
            .for_scope(&scope)
            .map_err(|_| HostFailure::Denied("storage-unavailable"))?;
        let grant = StorageGrant::from_needs(
            StorageNeeds {
                collections: &self.collections,
                ..StorageNeeds::default()
            },
            &GrantPolicy {
                writable: true,
                ..GrantPolicy::default()
            },
        )
        .map_err(|_| HostFailure::Denied("invalid-storage-grant"))?;
        Ok(ResolvedStorage {
            backend_identity: "tinychannels-module-artifact".to_owned(),
            binding_epoch: 1,
            canonical_scope: context.0.to_owned(),
            grant,
            server: Arc::new(StorageServer::new(storage)),
        })
    }

    async fn admit_write(
        &self,
        _context: &Profile,
        _module: &tinybus::module::ModuleId,
    ) -> Result<(), HostFailure> {
        Ok(())
    }
}

#[derive(Default)]
struct NoAudit;
impl AuditSink for NoAudit {
    fn record(&self, _event: AuditEvent) {}
}

fn fail(message: &'static str) -> Box<dyn Error> {
    std::io::Error::other(message).into()
}

pub async fn verify_prelease_migration(
    controller: &Connection,
    outsider: &Connection,
    host: &ModuleHost,
    info: &ModuleInfo,
    module_peer: &BusName,
) -> TestResult<()> {
    let migration_proxy = controller.proxy(
        module_peer.as_str(),
        MIGRATION_OBJECT_PATH,
        MIGRATION_INTERFACE,
    )?;
    let announcement: Value = migration_proxy
        .call_confidential(GET_SCHEMA_MEMBER, ())
        .await?;
    assert_eq!(announcement["declaration"]["owner"], "channels");
    assert_eq!(
        announcement["declaration"]["steps"]
            .as_array()
            .unwrap()
            .len(),
        14
    );
    assert!(
        info.manifest
            .provides
            .iter()
            .any(|provided| { provided.version.interface.as_str() == MIGRATION_INTERFACE })
    );
    assert!(
        migration_proxy
            .call::<Value>(GET_SCHEMA_MEMBER, ())
            .await
            .is_err()
    );

    let outsider_migration = outsider.proxy(
        module_peer.as_str(),
        MIGRATION_OBJECT_PATH,
        MIGRATION_INTERFACE,
    )?;
    let malformed = outsider_migration
        .call_confidential::<Value>(EXCHANGE_MEMBER, json!(17))
        .await
        .expect_err("outsider must be denied before malformed input is decoded");
    assert!(
        malformed
            .to_string()
            .contains("migration authority unavailable")
    );

    // Exercise the owner's actual step closures over a compiled module before
    // the broker has issued an ordinary storage lease.
    let module = MigrationModuleClient::discover(controller, module_peer).await?;
    let registry = Registry::compose_with_declarations(
        std::iter::empty(),
        std::iter::once(module.declaration().clone()),
    )?;
    let backend = MemoryStorage::new();
    registry.prepare(&backend).await?;
    let migration_scope = Scope::new("channels-migration-proof")?;
    let scoped = backend.for_scope(&migration_scope)?;
    scoped
        .documents()
        .put_with(
            "channel_remote_busy",
            "telegram:chat-42",
            json!({"legacy": false}),
            Precondition::Absent,
            WriteOptions::new().created_ms(1_700_000_000_000),
        )
        .await?;
    scoped
        .documents()
        .put(
            "whatsapp_identity",
            "account-a:identity-a",
            json!({"account":"account-a", "key":"identity-a", "value":"a2V5"}),
            Precondition::Absent,
        )
        .await?;
    let executor = MigrationDeliveryClient::new(Arc::new(module));
    let report = registry
        .migrate_with_executor(&scoped, &MigrateOptions::new(), &executor)
        .await?;
    assert_eq!(report.ran.len(), 14);
    registry.check_ready_in(&migration_scope, "channels.remote_busy")?;
    registry.check_ready_in(&migration_scope, "channels.whatsapp_identity")?;
    let busy_id = tinystoragedrivers_core::Key::of(["telegram", "chat-42"])?;
    let busy = scoped
        .documents()
        .get("channels.remote_busy", busy_id.as_str())
        .await?
        .ok_or_else(|| fail("remote busy transform did not create its target"))?;
    assert_eq!(busy.doc, json!({"busy": true}));
    assert_eq!(busy.meta.created_ms, 1_700_000_000_000);
    assert!(
        scoped
            .documents()
            .get("channel_remote_busy", "telegram:chat-42")
            .await?
            .is_none()
    );
    let identity_id = tinystoragedrivers_core::Key::of(["account-a", "identity-a"])?;
    let identity = scoped
        .documents()
        .get("channels.whatsapp_identity", identity_id.as_str())
        .await?
        .ok_or_else(|| fail("WhatsApp identity transform did not create its target"))?;
    assert_eq!(identity.doc["data_b64"], "a2V5");
    assert!(identity.doc.get("value").is_none());
    assert!(
        scoped
            .documents()
            .get("whatsapp_identity", "account-a:identity-a")
            .await?
            .is_none()
    );

    let invalid = json!({"storage_descriptor": {
        "service_peer": ":1.987654",
        "registration_id": 8,
        "object_root": "/org/tinystoragedrivers/Storage/Leases/8"
    }});
    assert!(host.reinitialize(&info.name, invalid).await.is_err());
    let channels = controller.proxy(
        module_peer.as_str(),
        tinychannels_bus::OBJECT_PATH,
        tinychannels_bus::BUS_NAME,
    )?;
    let names: Vec<String> = channels.call_confidential("ListChannels", ()).await?;
    assert!(
        names.is_empty(),
        "failed setup must leave the old Channels service live"
    );
    host.reinitialize(&info.name, json!({})).await?;
    Ok(())
}

pub struct CurrentStorageFixture {
    pub backend: Arc<MemoryStorage>,
    pub descriptor: StorageDescriptor,
    registration: StorageRegistration<StorageEnvironment, NoAudit, Profile>,
    lease: StorageLease,
}

impl CurrentStorageFixture {
    pub async fn revoke_default_lease(&self) -> bool {
        self.registration.revoke(&self.lease).await
    }

    pub async fn add_lease(
        &self,
        profile: &'static str,
        module_peer: &BusName,
    ) -> TestResult<(StorageLease, StorageDescriptor)> {
        self.registration
            .register_lease(
                HostStorageContext::new(Profile(profile)),
                module_peer.clone(),
            )
            .await
            .map_err(|_| fail("additional current-only storage lease registration failed"))
    }

    pub async fn revoke_lease(&self, lease: &StorageLease) -> bool {
        self.registration.revoke(lease).await
    }

    pub async fn shutdown(self) -> TestResult<()> {
        self.registration.shutdown().await?;
        Ok(())
    }
}

pub async fn register_current_storage(
    host: &ModuleHost,
    controller: &Connection,
    module_peer: &BusName,
) -> TestResult<CurrentStorageFixture> {
    let collections = [
        tinychannels::storage::schema::DELIVERY_PENDING,
        tinychannels::storage::schema::DELIVERY_FAILED,
        tinychannels::storage::schema::DELIVERY_RESULTS,
    ]
    .into_iter()
    .map(|name| CollectionSpec {
        name: name.to_owned(),
        indexes: Vec::new(),
        ttl_field: None,
        search: None,
    })
    .collect();
    let backend = Arc::new(MemoryStorage::new());
    let endpoint = GovernedStorage::new(
        Arc::new(StorageEnvironment {
            backend: Arc::clone(&backend),
            collections,
        }),
        Arc::new(NoAudit),
        GovernedEndpointLimits::default(),
    );
    let registration = attach_storage(
        host,
        controller,
        endpoint,
        tokio::runtime::Handle::current(),
    )
    .await
    .map_err(|_| fail("governed storage registration failed"))?;
    let (lease, descriptor) = registration
        .register_lease(
            HostStorageContext::new(Profile("channels-current-only")),
            module_peer.clone(),
        )
        .await
        .map_err(|_| fail("current-only storage lease registration failed"))?;
    Ok(CurrentStorageFixture {
        backend,
        descriptor,
        registration,
        lease,
    })
}
