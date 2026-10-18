use super::*;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::{Value, json};
use tinybus::{
    BusName, Error, Interface, InterfaceName, MemberName, ObjectPath, broker::Broker,
    module::{FailureReporter, ModuleRuntime, RetryPolicy, RuntimeNames},
    transport::memory::MemoryBus,
};

const HANDLE: &str = "0123456789abcdef0123456789abcdef";
const OBJECT: &str = "/ai/tinyhumans/tinychannels/Channels/runtime_0123456789abcdef0123456789abcdef";
const CALLBACK: &str = "/ai/tinyhumans/tinychannels/ChannelsHost/runtime_0123456789abcdef0123456789abcdef";

type Events = Arc<Mutex<Vec<String>>>;

struct LifecyclePeer {
    label: &'static str,
    connection: Connection,
    events: Events,
    reserve_started: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    reserve_release: Option<Arc<tokio::sync::Semaphore>>,
}

#[async_trait::async_trait]
impl Interface for LifecyclePeer {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(tinychannels_bus::BUS_NAME).expect("valid TinyChannels interface")
    }

    fn members(&self) -> Vec<MemberName> {
        [
            tinychannels_bus::methods::RESERVE_RUNTIME,
            tinychannels_bus::methods::START_RUNTIME,
            tinychannels_bus::methods::STOP_RUNTIME,
            "ChannelStatus",
        ]
        .into_iter()
        .map(|member| MemberName::new(member).expect("valid TinyChannels method"))
        .collect()
    }

    fn requires_confidential(&self, member: &MemberName) -> bool {
        matches!(
            member.as_str(),
            tinychannels_bus::methods::RESERVE_RUNTIME
                | tinychannels_bus::methods::START_RUNTIME
                | tinychannels_bus::methods::STOP_RUNTIME
        )
    }

    async fn call(&self, member: &MemberName, args: Value) -> tinybus::Result<Value> {
        self.events
            .lock()
            .expect("event lock")
            .push(format!("{}:{}", self.label, member.as_str()));
        match member.as_str() {
            tinychannels_bus::methods::RESERVE_RUNTIME => {
                let started = self
                    .reserve_started
                    .lock()
                    .expect("reserve lock")
                    .take();
                if let Some(started) = started {
                    let _ = started.send(());
                }
                if let Some(release) = &self.reserve_release {
                    release
                        .acquire()
                        .await
                        .map_err(|_| Error::failed("reservation gate closed"))?
                        .forget();
                }
                Ok(serde_json::to_value(tinychannels_bus::ChannelRuntimeReservation {
                    handle: HANDLE.into(),
                    object_path: OBJECT.into(),
                    host_object_path: CALLBACK.into(),
                })
                .expect("reservation serializes"))
            }
            tinychannels_bus::methods::START_RUNTIME => {
                let handle = args
                    .as_array()
                    .and_then(|values| values.first())
                    .and_then(Value::as_str);
                if handle != Some(HANDLE) {
                    return Err(Error::failed("unexpected start handle"));
                }
                let callback = self
                    .connection
                    .proxy(tinychannels_bus::HOST_BUS_NAME, CALLBACK, tinychannels_bus::HOST_BUS_NAME)
                    .map_err(|_| Error::failed("callback proxy unavailable"))?
                    .call::<String>("FixtureCallback", ())
                    .await?;
                if callback != "callback-live" {
                    return Err(Error::failed("callback was not registered"));
                }
                Ok(Value::Null)
            }
            tinychannels_bus::methods::STOP_RUNTIME => {
                let handle = args
                    .as_array()
                    .and_then(|values| values.first())
                    .and_then(Value::as_str);
                if handle != Some(HANDLE) {
                    return Err(Error::failed("unexpected stop handle"));
                }
                Ok(Value::Null)
            }
            "ChannelStatus" => Ok(json!(self.label)),
            _ => Err(Error::failed("unexpected TinyChannels method")),
        }
    }
}

#[derive(Clone)]
struct FixtureCallback;

#[async_trait::async_trait]
impl Interface for FixtureCallback {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(tinychannels_bus::HOST_BUS_NAME).expect("valid callback interface")
    }

    fn members(&self) -> Vec<MemberName> {
        vec![MemberName::new("FixtureCallback").expect("valid callback method")]
    }

    async fn call(&self, _: &MemberName, _: Value) -> tinybus::Result<Value> {
        Ok(json!("callback-live"))
    }
}

struct StrictAccess {
    connection: Connection,
    record: &'static ReleaseRecord,
    reporter: FailureReporter,
}

impl ModuleAccess for StrictAccess {
    fn find(&self, id: &str) -> Option<&'static ReleaseRecord> {
        (id == self.record.id).then_some(self.record)
    }

    fn enabled(&self) -> bool {
        true
    }

    fn reporter(&self) -> &FailureReporter {
        &self.reporter
    }

    fn ensure_loaded<'a>(&'a self, _: &'a str) -> AccessFuture<'a, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn prepare_attested<'a>(&'a self, _: &'a str) -> AccessFuture<'a, Result<(), String>> {
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

fn attested_record() -> &'static ReleaseRecord {
    let host_key = Box::leak(
        tinybus::module::platform::host_candidates()
            .into_iter()
            .next()
            .expect("supported TinyBus host")
            .into_boxed_str(),
    );
    let archive = Box::leak("tinychannels-runtime-fixture.tar.gz".to_owned().into_boxed_str());
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
    Box::leak(Box::new(ReleaseRecord {
        id: MODULE_ID,
        bus_name: tinychannels_bus::BUS_NAME,
        object_path: tinychannels_bus::OBJECT_PATH,
        version: "runtime-fixture",
        release_url: "https://example.invalid/tinychannels-runtime-fixture",
        assets,
    }))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn runtime_lease_stays_on_the_attested_peer_across_alias_handover() {
    let bus = MemoryBus::new();
    let broker = Broker::new();
    let broker_task = broker.clone().spawn(bus.clone());
    let service_a = Connection::connect(bus.connect().await.expect("A transport"))
        .await
        .expect("A connection");
    let service_b = Connection::connect(bus.connect().await.expect("B transport"))
        .await
        .expect("B connection");
    let caller = Connection::connect(bus.connect().await.expect("caller transport"))
        .await
        .expect("caller connection");
    let record = attested_record();
    let events = Arc::new(Mutex::new(Vec::new()));
    let (reserve_started, reserve_seen) = tokio::sync::oneshot::channel();
    let reserve_release = Arc::new(tokio::sync::Semaphore::new(0));

    service_a
        .serve_at(
            ObjectPath::new(record.object_path).expect("module path"),
            LifecyclePeer {
                label: "peer-a",
                connection: service_a.clone(),
                events: events.clone(),
                reserve_started: Mutex::new(Some(reserve_started)),
                reserve_release: Some(reserve_release.clone()),
            },
        )
        .await
        .expect("serve A");
    service_b
        .serve_at(
            ObjectPath::new(record.object_path).expect("module path"),
            LifecyclePeer {
                label: "peer-b",
                connection: service_b.clone(),
                events: events.clone(),
                reserve_started: Mutex::new(None),
                reserve_release: None,
            },
        )
        .await
        .expect("serve B");
    service_a
        .request_name(record.bus_name)
        .await
        .expect("A owns alias");
    broker.attest_module(
        &service_a.unique_name().expect("A unique bus name"),
        tinybus::attest::Attestation {
            name: BusName::new(record.bus_name).expect("module bus name"),
            sha256: record.assets[0].sha256.to_owned(),
            library_sha256: record.assets[0].library_sha256.map(str::to_owned),
        },
    );

    let client = TinyChannelsClient::new(ModuleClient::new(StrictAccess {
        connection: caller,
        record,
        reporter: FailureReporter::new(Arc::new(NoFailures)),
    }));
    let driver = client.runtime_driver().await.expect("bind attested A once");
    let runtime = ModuleRuntime::start(&RuntimeNames::default(), None).expect("lease runtime");
    let opening = runtime.lease(
        driver,
        |_, _| Ok(FixtureCallback),
        RetryPolicy::new(Duration::from_millis(20), vec![]).expect("valid retry policy"),
        |_| {},
    );
    let opening = tokio::spawn(opening);

    // Binding precedes the confidential reservation. Moving the well-known
    // name while A is paused must not redirect the handle or object to B.
    reserve_seen.await.expect("A received ReserveRuntime");
    service_a.release_name(record.bus_name).await.expect("release A alias");
    service_b.request_name(record.bus_name).await.expect("B takes alias");
    reserve_release.add_permits(1);

    let lease = opening.await.expect("lease task").expect("lease activates");
    assert_eq!(
        lease
            .object()
            .channel_status("discord")
            .await
            .expect("same bound object's business call"),
        "peer-a"
    );
    assert!(matches!(lease.close().await, Ok(())), "STOP ACK closes lease");
    assert_eq!(
        *events.lock().expect("event log"),
        [
            "peer-a:ReserveRuntime",
            "peer-a:StartRuntime",
            "peer-a:ChannelStatus",
            "peer-a:StopRuntime",
        ]
    );

    drop(service_a);
    assert!(lease.object().channel_status("discord").await.is_err());
    assert_eq!(
        *events.lock().expect("event log after disconnect"),
        [
            "peer-a:ReserveRuntime",
            "peer-a:StartRuntime",
            "peer-a:ChannelStatus",
            "peer-a:StopRuntime",
        ]
    );
    broker_task.abort();
}
