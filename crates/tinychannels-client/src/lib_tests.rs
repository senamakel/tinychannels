use super::*;

use std::{
    sync::{Arc, Mutex},
};

use serde_json::json;
use tinybus::{
    Connection, InterfaceName, MemberName, ObjectPath,
    broker::Broker,
    module::{
        AccessFuture, FailureReport, FailureReporter, FailureSink, ModuleAccess,
        ProxyFailure, ReleaseAsset, ReleaseRecord,
    },
    service::Interface,
    transport::memory::MemoryBus,
};

const MODULE_ID: &str = "tinychannels";
type Calls = Arc<Mutex<Vec<String>>>;

struct NoFailures;

impl FailureSink for NoFailures {
    fn capture(&self, _report: &FailureReport<'_>) {}
}

struct RecordingModule(Calls);

#[async_trait::async_trait]
impl Interface for RecordingModule {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(tinychannels_bus::BUS_NAME).expect("valid bus interface")
    }

    fn members(&self) -> Vec<MemberName> {
        tinychannels_bus::METHODS
            .iter()
            .map(|name| MemberName::new(*name).expect("valid TinyChannels method"))
            .collect()
    }

    async fn call(&self, member: &MemberName, _args: serde_json::Value) -> tinybus::Result<serde_json::Value> {
        self.0
            .lock()
            .map_err(|_| tinybus::Error::failed("call log poisoned"))?
            .push(member.as_str().to_owned());
        Ok(json!({"result":"discord_guilds","value":{"raw":[]}}))
    }
}

struct TestAccess {
    record: &'static ReleaseRecord,
    connection: Connection,
    reporter: FailureReporter,
}

impl ModuleAccess for TestAccess {
    fn find(&self, id: &str) -> Option<&'static ReleaseRecord> {
        (id == self.record.id).then_some(self.record)
    }

    fn enabled(&self) -> bool {
        true
    }

    fn reporter(&self) -> &FailureReporter {
        &self.reporter
    }

    fn ensure_loaded<'a>(&'a self, _id: &'a str) -> AccessFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn prepare_attested<'a>(&'a self, _id: &'a str) -> AccessFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn proxy<'a>(
        &'a self,
        bus_name: &'a str,
        object_path: &'a str,
    ) -> AccessFuture<'a, Result<tinybus::Proxy, ProxyFailure>> {
        Box::pin(async move {
            self.connection
                .proxy(bus_name, object_path, bus_name)
                .map_err(|_| ProxyFailure::Contract)
        })
    }
}

async fn unattested_client() -> (TinyChannelsClient, Calls) {
    let host_key = Box::leak(
        tinybus::module::platform::host_candidates()
            .into_iter()
            .next()
            .expect("supported TinyBus host")
            .into_boxed_str(),
    );
    let archive = Box::leak("fixture.tar.gz".to_owned().into_boxed_str());
    let archive_sha = Box::leak("a".repeat(64).into_boxed_str());
    let library_sha = Box::leak("b".repeat(64).into_boxed_str());
    let assets = Box::leak(
        vec![ReleaseAsset {
            host_key,
            archive,
            sha256: archive_sha,
            library_sha256: Some(library_sha),
        }]
        .into_boxed_slice(),
    );
    let record = Box::leak(Box::new(ReleaseRecord {
        id: MODULE_ID,
        bus_name: tinychannels_bus::BUS_NAME,
        object_path: tinychannels_bus::OBJECT_PATH,
        version: "0.1.36",
        release_url: "https://example.invalid/tinychannels",
        assets,
    }));

    let calls = Arc::new(Mutex::new(Vec::new()));
    let bus = MemoryBus::new();
    let _broker = Broker::new().spawn(bus.clone());
    let service = Connection::connect(bus.connect().await.expect("service transport"))
        .await
        .expect("service connection");
    service
        .serve_at(
            ObjectPath::new(tinychannels_bus::OBJECT_PATH).expect("valid module path"),
            RecordingModule(Arc::clone(&calls)),
        )
        .await
        .expect("serve fixture");
    service
        .request_name(tinychannels_bus::BUS_NAME)
        .await
        .expect("own module name");
    let caller = Connection::connect(bus.connect().await.expect("caller transport"))
        .await
        .expect("caller connection");
    let module = ModuleClient::new(TestAccess {
        record,
        connection: caller,
        reporter: FailureReporter::new(Arc::new(NoFailures)),
    });
    (TinyChannelsClient::new(module), calls)
}

#[test]
fn discord_replies_decode_only_the_matching_raw_projection() {
    let response: ChannelControllerResult = serde_json::from_value(json!({
        "result":"discord_guilds",
        "value":{"raw":[{"id":"guild-1","name":"Server","icon":null}]}
    }))
    .unwrap();
    let guilds: Vec<DiscordGuild> =
        TinyChannelsClient::decode_discord(response, DiscordReply::Guilds).unwrap();
    assert_eq!(guilds[0].id, "guild-1");

    let wrong_projection: ChannelControllerResult = serde_json::from_value(json!({
        "result":"discord_channels",
        "value":{"raw":[]}
    }))
    .unwrap();
    assert!(TinyChannelsClient::decode_discord::<Vec<DiscordGuild>>(
        wrong_projection,
        DiscordReply::Guilds
    )
    .is_none());

    let malformed: ChannelControllerResult = serde_json::from_value(json!({
        "result":"discord_guilds",
        "value":{"raw":[{"id":"guild-1"}]}
    }))
    .unwrap();
    assert!(TinyChannelsClient::decode_discord::<Vec<DiscordGuild>>(
        malformed,
        DiscordReply::Guilds
    )
    .is_none());
}

#[tokio::test]
async fn discord_credentials_never_fall_back_when_peer_proof_is_missing() {
    let (client, calls) = unattested_client().await;
    let result = client
        .discord_list_guilds(
            ChannelsConfig::default(),
            "fixture-private-token".into(),
            HttpProxyPolicy::default(),
        )
        .await;

    let error = result.expect_err("unattested module must be rejected");
    assert_eq!(error.kind, ModuleCallError::Unavailable);
    assert!(!format!("{error:?}").contains("fixture-private-token"));
    assert!(!error.to_string().contains("fixture-private-token"));
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn detailed_remote_invalid_input_is_available_only_in_explicit_fields() {
    let failure = ModuleCallFailure {
        kind: ModuleCallError::ModuleFault,
        wire_name: "ai.tinyhumans.tinychannels.Error.InvalidInput".into(),
        message: "invalid request containing fixture-private-token".into(),
    };

    assert_eq!(
        failure.wire_name,
        "ai.tinyhumans.tinychannels.Error.InvalidInput"
    );
    assert!(failure.message.contains("fixture-private-token"));
    assert!(!format!("{failure:?}").contains("fixture-private-token"));
    assert!(!failure.to_string().contains("fixture-private-token"));
}
