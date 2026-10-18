//! Cold fixture readiness is separate from bounded runtime operations.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::Duration;

mod support;

use tinybus::module::ModuleCallError;
use tinychannels_client::ChannelRuntimeErrorKind;

#[tokio::test(start_paused = true)]
async fn cold_bind_waits_for_ready_and_fails_closed_when_ready_is_withheld() {
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
    wait_for_readiness_attempt(&fixture, before_limited_bind).await;
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

    // A default client has no extra caller deadline. It waits on the same cold
    // setup barrier, then binds successfully after the fixture publishes READY.
    let startup_client = fixture.startup_client.clone();
    let before_startup_bind = fixture.readiness_attempts();
    let startup_bind = tokio::spawn(async move { startup_client.runtime_driver().await });
    wait_for_readiness_attempt(&fixture, before_startup_bind).await;
    assert!(
        !startup_bind.is_finished(),
        "cold default-deadline bind remains pending at the setup barrier"
    );
    fixture.release_ready();
    let driver = startup_bind
        .await
        .expect("default-deadline bind task completes")
        .expect("cold attested bind succeeds after setup publishes READY");
    assert!(
        fixture.module_ready(),
        "host observes READY for the fixture bus"
    );
    drop(driver);
}

async fn wait_for_readiness_attempt(fixture: &support::RuntimeFixture, previous: usize) {
    for _ in 0..10_000 {
        if fixture.readiness_attempts() > previous {
            assert_eq!(fixture.bind_stage(), "readiness_in_progress");
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("module client never entered host readiness preparation");
}
