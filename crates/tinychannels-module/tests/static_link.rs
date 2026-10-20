//! The statically linked build is admitted by a `ModuleHost` and reaches `Ready`.

#![cfg(feature = "static-link")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use tinybus::module::{ModuleRuntime, ModuleState, RuntimeNames};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn linked_module_is_admitted_and_becomes_ready() {
    let runtime = ModuleRuntime::start(&RuntimeNames::default(), None).unwrap();
    let host = runtime.host();
    let connection = runtime.connection();
    let info = host
        .attach_linked_with_config(
            tinychannels_module::linked_module().unwrap(),
            serde_json::json!({}),
        )
        .expect("linked module is admitted");
    assert_eq!(info.name, "tinychannels-module");
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if host
                .list()
                .iter()
                .any(|module| matches!(module.state, ModuleState::Ready))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("linked module reaches Ready");
    let module_peer = connection
        .name_owner(info.manifest.bus_name.as_str())
        .await
        .unwrap()
        .unwrap();
    let migration = connection
        .proxy(
            module_peer.as_str(),
            tinystoragedrivers_tinybus::migration::MIGRATION_OBJECT_PATH,
            tinystoragedrivers_tinybus::migration::MIGRATION_INTERFACE,
        )
        .unwrap();
    let announcement: serde_json::Value = migration
        .call_confidential(tinystoragedrivers_tinybus::migration::GET_SCHEMA_MEMBER, ())
        .await
        .unwrap();
    assert_eq!(announcement["declaration"]["owner"], "channels");
    assert_eq!(
        announcement["declaration"]["steps"]
            .as_array()
            .unwrap()
            .len(),
        14
    );
}
