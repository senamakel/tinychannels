//! Cold fixture readiness is separate from bounded runtime operations.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::Duration;

mod support;

use tinybus::module::ModuleCallError;
use tinychannels_client::ChannelRuntimeErrorKind;

#[tokio::test(start_paused = true)]
async fn withheld_ready_fails_closed_within_the_client_deadline() {
    // One ModuleRuntime owns the fixture and its gate for this process. The
    // module serves its object and claims its name before entering the host
    // callback gate, but TinyBus cannot publish READY until setup returns.
    let fixture = support::gated_runtime_fixture().await;
    assert!(
        !fixture.module_ready(),
        "host must not report gated A as READY"
    );

    let limited_client = fixture.client.clone();
    let before_limited_bind = fixture.readiness_attempts();
    let limited_bind = tokio::spawn(async move { limited_client.runtime_driver().await });
    fixture
        .wait_for_readiness_attempt(before_limited_bind)
        .await;
    assert!(
        !limited_bind.is_finished(),
        "a live fixture object is not a completed attested bind while READY is withheld"
    );

    // This exercises the existing two-second caller deadline with virtual
    // host time. The fixture worker uses a separate Tokio runtime and remains
    // deterministically blocked at its semaphore until released below.
    tokio::time::advance(Duration::from_secs(2)).await;
    let error = limited_bind
        .await
        .expect("bounded bind task completes")
        .err()
        .expect("withheld READY cannot be accepted");
    assert_eq!(
        error.kind(),
        ChannelRuntimeErrorKind::Module(ModuleCallError::Unavailable)
    );
    assert!(
        !fixture.module_ready(),
        "timed-out bind leaves fixture unready"
    );

    // Release the native setup task even on the negative path so its foreign
    // Tokio runtime and the host callback do not remain blocked at process exit.
    fixture.release_ready();
    fixture.wait_until_gate_callback_returns().await;
    fixture
        .runtime
        .host()
        .ensure_ready(tinychannels_bus::BUS_NAME)
        .await
        .expect("released readiness gate lets fixture setup finish cleanly");
}
