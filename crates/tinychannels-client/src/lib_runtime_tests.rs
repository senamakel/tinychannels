use super::*;

use tinybus::module::RuntimeLeaseDriver;

#[test]
fn runtime_driver_public_type_implements_the_sdk_lease_contract() {
    fn assert_driver<T: RuntimeLeaseDriver>() {}

    assert_driver::<ChannelRuntimeDriver>();
}

#[test]
fn runtime_errors_keep_typed_causes_without_remote_payloads() {
    let error = ChannelRuntimeError::module(
        ChannelRuntimeOperation::Stop,
        tinybus::module::ModuleCallError::ModuleFault,
    );
    assert_eq!(error.operation(), ChannelRuntimeOperation::Stop);
    assert_eq!(
        error.kind(),
        ChannelRuntimeErrorKind::Module(tinybus::module::ModuleCallError::ModuleFault)
    );
    assert!(std::error::Error::source(&error).is_some());
    assert!(!format!("{error:?}").contains("private remote payload"));
    assert!(!error.to_string().contains("private remote payload"));
}

#[tokio::test]
async fn runtime_driver_builders_bind_strictly_before_any_runtime_call() {
    let (client, calls) = unattested_client().await;

    // The peer is alive and ordinary calls work, so refusing the runtime API
    // proves strict binding rather than a missing module or bus connection.
    let ready: bool = client
        .module
        .call(MODULE_ID, "FixtureReady", ())
        .await
        .unwrap_or_else(|error| {
            panic!(
                "ordinary readiness control succeeds on the live peer: {error:?}; calls={:?}",
                calls.lock().expect("call log")
            )
        });
    assert!(ready);
    let before = calls.lock().expect("call log").clone();
    assert_eq!(before, ["FixtureReady"]);

    assert!(client.runtime_driver().await.is_err());
    assert!(
        client
            .runtime_driver_with_dispatch(tinychannels_bus::ChannelRuntimeDispatchPolicy {
                listener_count: 1
            },)
            .await
            .is_err()
    );

    // Failed proof must occur before ReserveRuntime or any business method is
    // delivered. In particular, availability must never trigger fallback.
    assert_eq!(*calls.lock().expect("call log after refusal"), before);
}
