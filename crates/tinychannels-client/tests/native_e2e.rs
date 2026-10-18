//! Public client routes over a strictly admitted cached native fixture.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

mod support;

use tinybus::{
    BusName, Error, Interface, InterfaceName, MemberName, ObjectPath,
    module::{ModuleCallError, ModuleCallFailure, RetryPolicy},
};
use tinychannels_bus::{
    ChannelControlRequest, ChannelControlResult, ChannelHealthState, ChannelInteraction,
    ChannelInteractionRequest, ChannelInteractionResult, ChannelMessagePreparation,
    ChannelOutboundIntent, ChannelRuntimeDispatchPolicy, ChannelSupervisionPolicy, ChannelsConfig,
    HttpProxyPolicy, ProgressiveCommand, ProgressiveResult, RelayReservation,
    config::DiscordConfig,
};

const TOKEN: &str = "fixture-call-secret";
const RUNTIME_HANDLE: &str = "0123456789abcdef0123456789abcdef";
const RUNTIME_OBJECT_PATH: &str =
    "/ai/tinyhumans/tinychannels/Channels/runtime_0123456789abcdef0123456789abcdef";
const RUNTIME_CALLBACK_PATH: &str =
    "/ai/tinyhumans/tinychannels/ChannelsHost/runtime_0123456789abcdef0123456789abcdef";

fn config() -> ChannelsConfig {
    ChannelsConfig {
        cli: false,
        message_timeout_secs: 47,
        active_channel: Some("discord".into()),
        discord: Some(DiscordConfig {
            bot_token: "fixture-config-secret".into(),
            guild_id: Some("config-guild".into()),
            channel_id: Some("config-channel".into()),
            allowed_users: vec!["member-1".into()],
            listen_to_bots: true,
            mention_only: true,
        }),
        ..Default::default()
    }
}

fn proxy(selector: &str) -> HttpProxyPolicy {
    HttpProxyPolicy {
        http_proxy: Some("http://proxy.example.test:8080".into()),
        https_proxy: Some("https://proxy.example.test:8443".into()),
        all_proxy: Some("socks5://proxy.example.test:1080".into()),
        no_proxy: vec!["localhost".into(), ".internal".into()],
        service_selectors: vec![selector.into()],
    }
}

fn assert_one_module_fault(reports: &support::Reports) {
    assert_eq!(
        reports.entries(),
        [(
            "tinychannels".into(),
            "0.1.36".into(),
            "module_fault".into()
        )]
    );
}

fn assert_sanitized_failure(failure: &ModuleCallFailure) {
    assert_eq!(failure.kind, ModuleCallError::ModuleFault);
    assert!(!format!("{failure:?}").contains("fixture-private-token"));
    assert!(!failure.to_string().contains("fixture-private-token"));
}

#[tokio::test]
async fn all_public_wrappers_route_to_the_pinned_peer_and_decode_typed_results() {
    let fixture = support::fixture().await;
    let record = support::record(support::Pin::Correct);
    let before = support::calls(&fixture.probe).await;
    // Observe the admitted peer's public proof metadata; each actual operation
    // below still traverses TinyChannelsClient's strict ModuleClient call.
    let peer = fixture
        .probe
        .name_owner(record.bus_name)
        .await
        .expect("module owner lookup")
        .expect("fixture is serving");
    assert!(peer.is_unique());
    let proof = fixture
        .probe
        .attestation(peer)
        .await
        .expect("public peer proof lookup")
        .expect("cached release is attested");
    assert_eq!(proof.name.as_str(), record.bus_name);
    assert_eq!(proof.sha256, record.assets[0].sha256);
    assert_eq!(
        proof.library_sha256.as_deref(),
        record.assets[0].library_sha256
    );

    let guilds = fixture
        .client
        .discord_list_guilds(config(), TOKEN.into(), proxy("channel.discord"))
        .await
        .expect("guild list from the pinned native fixture");
    assert_eq!(guilds.len(), 1);
    assert_eq!(guilds[0].id, "guild-1");
    assert_eq!(guilds[0].name, "Fixture guild");
    assert_eq!(guilds[0].icon, None);

    let channels = fixture
        .client
        .discord_list_channels(
            config(),
            "guild-1".into(),
            TOKEN.into(),
            proxy("channel.discord"),
        )
        .await
        .expect("channel list from the pinned native fixture");
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].id, "channel-1");
    assert_eq!(channels[0].name, "general");
    assert_eq!(channels[0].channel_type, 0);
    assert_eq!(channels[0].position, 11);
    assert_eq!(channels[0].parent_id.as_deref(), Some("category-1"));

    let permissions = fixture
        .client
        .discord_check_permissions(
            config(),
            "guild-1".into(),
            "channel-1".into(),
            TOKEN.into(),
            proxy("channel.discord"),
        )
        .await
        .expect("permission check from the pinned native fixture");
    assert!(permissions.can_view_channel);
    assert!(!permissions.can_send_messages);
    assert!(permissions.can_read_message_history);
    assert_eq!(permissions.missing_permissions, ["SEND_MESSAGES"]);

    let health = fixture
        .client
        .check_health(config(), proxy("channel.discord"), 12_345)
        .await
        .expect("health response from the pinned native fixture");
    assert_eq!(health[0].channel, "discord");
    assert_eq!(health[0].state, ChannelHealthState::Healthy);
    assert_eq!(health[1].channel, "telegram");
    assert_eq!(health[1].state, ChannelHealthState::Timeout);
    assert_eq!(support::calls(&fixture.probe).await, before + 4);

    // The fixture validates full config, bot token, IDs, every proxy field,
    // and the non-default timeout before returning these typed results.
    assert_eq!(record.version, "0.1.36");
}

#[tokio::test]
async fn malformed_public_responses_report_one_sanitized_module_fault() {
    let fixture = support::fixture().await;

    fixture.reports.clear();
    let guilds = fixture
        .client
        .discord_list_guilds(config(), TOKEN.into(), proxy("test:wrong-variant"))
        .await
        .expect_err("wrong result variant is malformed");
    assert_sanitized_failure(&guilds);
    assert_one_module_fault(&fixture.reports);

    fixture.reports.clear();
    let channels = fixture
        .client
        .discord_list_channels(
            config(),
            "guild-1".into(),
            TOKEN.into(),
            proxy("test:missing-raw"),
        )
        .await
        .expect_err("missing raw projection is malformed");
    assert_sanitized_failure(&channels);
    assert_one_module_fault(&fixture.reports);

    fixture.reports.clear();
    let permissions = fixture
        .client
        .discord_check_permissions(
            config(),
            "guild-1".into(),
            "channel-1".into(),
            TOKEN.into(),
            proxy("test:malformed-payload"),
        )
        .await
        .expect_err("wrong permission payload shape is malformed");
    assert_sanitized_failure(&permissions);
    assert_one_module_fault(&fixture.reports);

    fixture.reports.clear();
    let health = fixture
        .client
        .check_health(config(), proxy("test:malformed-payload"), 12_345)
        .await
        .expect_err("wrong health payload shape is malformed");
    assert_eq!(health.kind, ModuleCallError::IncompatibleContract);
    assert!(!format!("{health:?}").contains("fixture-private-token"));
    assert!(!health.to_string().contains("fixture-private-token"));
    assert_eq!(
        fixture.reports.entries(),
        [(
            "tinychannels".into(),
            "0.1.36".into(),
            "incompatible_contract".into()
        )]
    );
}

#[tokio::test]
async fn detailed_invalid_input_keeps_its_wire_classification_and_redacts_rendering() {
    let fixture = support::fixture().await;
    fixture.reports.clear();
    let failure = fixture
        .client
        .discord_list_guilds(config(), TOKEN.into(), proxy("test:invalid-input"))
        .await
        .expect_err("fixture emits an InvalidInput remote failure");

    assert_eq!(failure.kind, ModuleCallError::ModuleFault);
    assert_eq!(
        failure.wire_name,
        "ai.tinyhumans.tinychannels.Error.InvalidInput"
    );
    assert!(failure.message.contains("fixture-private-token"));
    assert!(!format!("{failure:?}").contains("fixture-private-token"));
    assert!(!failure.to_string().contains("fixture-private-token"));
    assert_one_module_fault(&fixture.reports);
}

#[tokio::test]
async fn wrong_release_pins_and_unattested_peers_are_refused_before_business_calls() {
    let fixture = support::fixture().await;
    let before = support::calls(&fixture.probe).await;

    for client in [&fixture.wrong_archive_client, &fixture.wrong_library_client] {
        let error = client
            .discord_list_guilds(config(), TOKEN.into(), proxy("channel.discord"))
            .await
            .expect_err("a mismatched half of the paired pin must be refused");
        support::assert_unavailable(error);
        assert_eq!(support::calls(&fixture.probe).await, before);

        let error = client
            .discord_list_channels(
                config(),
                "guild-1".into(),
                TOKEN.into(),
                proxy("channel.discord"),
            )
            .await
            .expect_err("a mismatched pin must refuse channel listing");
        support::assert_unavailable(error);
        assert_eq!(support::calls(&fixture.probe).await, before);

        let error = client
            .discord_check_permissions(
                config(),
                "guild-1".into(),
                "channel-1".into(),
                TOKEN.into(),
                proxy("channel.discord"),
            )
            .await
            .expect_err("a mismatched pin must refuse permission checks");
        support::assert_unavailable(error);
        assert_eq!(support::calls(&fixture.probe).await, before);

        let error = client
            .check_health(config(), proxy("channel.discord"), 12_345)
            .await
            .expect_err("a mismatched pin must refuse health checks");
        support::assert_unavailable(error);
        assert_eq!(support::calls(&fixture.probe).await, before);
    }

    let unattested = support::unattested_fixture().await;
    let owner = support::assert_unattested_peer_live(&unattested.probe).await;
    let error = unattested
        .client
        .discord_list_guilds(config(), TOKEN.into(), proxy("channel.discord"))
        .await
        .expect_err("unique but unattested peer must be refused");
    assert_eq!(error.kind, ModuleCallError::Unavailable);
    assert_eq!(
        unattested.calls.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        support::assert_unattested_peer_live(&unattested.probe).await,
        owner
    );

    let error = unattested
        .client
        .discord_list_channels(
            config(),
            "guild-1".into(),
            TOKEN.into(),
            proxy("channel.discord"),
        )
        .await
        .expect_err("unique but unattested peer must refuse channel listing");
    assert_eq!(error.kind, ModuleCallError::Unavailable);
    assert_eq!(
        unattested.calls.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        support::assert_unattested_peer_live(&unattested.probe).await,
        owner
    );

    let error = unattested
        .client
        .discord_check_permissions(
            config(),
            "guild-1".into(),
            "channel-1".into(),
            TOKEN.into(),
            proxy("channel.discord"),
        )
        .await
        .expect_err("unique but unattested peer must refuse permission checks");
    assert_eq!(error.kind, ModuleCallError::Unavailable);
    assert_eq!(
        unattested.calls.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        support::assert_unattested_peer_live(&unattested.probe).await,
        owner
    );

    let error = unattested
        .client
        .check_health(config(), proxy("channel.discord"), 12_345)
        .await
        .expect_err("unique but unattested peer must refuse health checks");
    assert_eq!(error.kind, ModuleCallError::Unavailable);
    assert_eq!(
        unattested.calls.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    assert_eq!(
        support::assert_unattested_peer_live(&unattested.probe).await,
        owner
    );
}

#[derive(Clone)]
struct FixtureCallback;

#[async_trait::async_trait]
impl Interface for FixtureCallback {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(tinychannels_bus::HOST_BUS_NAME)
            .expect("TinyChannels host callback interface")
    }

    fn members(&self) -> Vec<MemberName> {
        vec![MemberName::new("FixtureCallback").expect("fixture callback method")]
    }

    async fn call(
        &self,
        _: &MemberName,
        _: serde_json::Value,
    ) -> tinybus::Result<serde_json::Value> {
        Ok(serde_json::json!("callback-live"))
    }
}

struct UnattestedReplacementRoot(Arc<AtomicUsize>);

#[async_trait::async_trait]
impl Interface for UnattestedReplacementRoot {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(tinychannels_bus::BUS_NAME).expect("TinyChannels module interface")
    }

    fn members(&self) -> Vec<MemberName> {
        vec![MemberName::new("FixtureReady").expect("readiness method")]
    }

    async fn call(
        &self,
        member: &MemberName,
        _: serde_json::Value,
    ) -> tinybus::Result<serde_json::Value> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if member.as_str() == "FixtureReady" {
            Ok(serde_json::json!(true))
        } else {
            Err(Error::failed("unexpected replacement root call"))
        }
    }
}

struct UnattestedReplacementObject(Arc<AtomicUsize>);

#[async_trait::async_trait]
impl Interface for UnattestedReplacementObject {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(tinychannels_bus::BUS_NAME).expect("TinyChannels module interface")
    }

    fn members(&self) -> Vec<MemberName> {
        vec![
            MemberName::new(tinychannels_bus::methods::CHANNEL_STATUS)
                .expect("channel status method"),
        ]
    }

    async fn call(
        &self,
        member: &MemberName,
        _: serde_json::Value,
    ) -> tinybus::Result<serde_json::Value> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if member.as_str() == tinychannels_bus::methods::CHANNEL_STATUS {
            Ok(serde_json::json!("peer-b"))
        } else {
            Err(Error::failed("unexpected replacement object call"))
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn runtime_driver_keeps_calls_on_attested_a_after_ordinary_unattested_b_takes_alias() {
    let fixture = support::runtime_fixture().await;
    let driver = fixture
        .client
        .runtime_driver()
        .await
        .expect("strictly bind TinyChannels A before reservation");
    let control = fixture
        .control
        .bind_attested("tinychannels")
        .await
        .expect("hold a test control reference to A before alias handover");
    let ready: bool = control
        .call("FixtureReady", ())
        .await
        .expect("A is live before reservation");
    assert!(ready);
    assert!(
        control
            .call::<()>(tinychannels_bus::methods::START_RUNTIME, (RUNTIME_HANDLE,))
            .await
            .is_err(),
        "the native fixture refuses an ordinary lifecycle call"
    );
    control
        .call::<()>("FixtureReleaseAlias", ())
        .await
        .expect("A releases its well-known name");

    // `serve_host_object` is an ordinary, unattested host service. It is used
    // only to prove alias availability; it cannot receive confidential calls.
    let replacement_calls = Arc::new(AtomicUsize::new(0));
    let _replacement = fixture
        .runtime
        .serve_host_object(
            BusName::new(tinychannels_bus::BUS_NAME).expect("module bus name"),
            ObjectPath::new(tinychannels_bus::OBJECT_PATH).expect("module root path"),
            UnattestedReplacementRoot(Arc::clone(&replacement_calls)),
        )
        .await
        .expect("ordinary B can claim the released module alias");
    fixture
        .runtime
        .connection()
        .serve_at(
            ObjectPath::new(RUNTIME_OBJECT_PATH).expect("fixture runtime object path"),
            UnattestedReplacementObject(Arc::clone(&replacement_calls)),
        )
        .await
        .expect("B has the same private path as a replacement canary");

    let b_ready: bool = fixture
        .runtime
        .connection()
        .proxy(
            tinychannels_bus::BUS_NAME,
            tinychannels_bus::OBJECT_PATH,
            tinychannels_bus::BUS_NAME,
        )
        .expect("ordinary B root proxy")
        .call("FixtureReady", ())
        .await
        .expect("ordinary readiness call proves B is available");
    assert!(b_ready);
    let b_object: String = fixture
        .runtime
        .connection()
        .proxy(
            tinychannels_bus::BUS_NAME,
            RUNTIME_OBJECT_PATH,
            tinychannels_bus::BUS_NAME,
        )
        .expect("ordinary B object proxy")
        .call(tinychannels_bus::methods::CHANNEL_STATUS, ("discord",))
        .await
        .expect("ordinary status call proves the colliding B path is live");
    assert_eq!(b_object, "peer-b");
    let b_calls_before_lease = replacement_calls.load(Ordering::SeqCst);

    let lease = fixture
        .runtime
        .lease(
            driver,
            |_, _| Ok(FixtureCallback),
            RetryPolicy::new(Duration::from_secs(2), vec![]).expect("valid stop policy"),
            |_| {},
        )
        .await
        .expect("reserve, export callback and start on the already-bound A");
    let ordinary_object = control
        .object_client(RUNTIME_OBJECT_PATH)
        .expect("test-only held object on A");
    assert!(
        ordinary_object
            .call::<Vec<tinychannels_bus::module_runtime::ConfiguredChannelInfo>>(
                tinychannels_bus::methods::DESCRIBE_CONFIGURED_CHANNELS,
                (ChannelsConfig::default(), HttpProxyPolicy::default()),
            )
            .await
            .is_err(),
        "the native fixture refuses a confidential object call sent ordinarily"
    );
    assert!(
        control
            .call::<()>(tinychannels_bus::methods::STOP_RUNTIME, (RUNTIME_HANDLE,))
            .await
            .is_err(),
        "the native fixture refuses an ordinary STOP"
    );
    assert_eq!(
        lease
            .object()
            .channel_status("discord".into())
            .await
            .expect("typed object call stays on A after alias handover"),
        "peer-a"
    );
    assert_eq!(
        replacement_calls.load(Ordering::SeqCst),
        b_calls_before_lease,
        "B's live colliding object received no call from the lease"
    );

    let providers = lease
        .object()
        .describe_configured_channels(ChannelsConfig::default(), HttpProxyPolicy::default())
        .await
        .expect("confidential provider description");
    assert!(providers.is_empty());
    lease
        .object()
        .start_channel_supervised(
            "discord".into(),
            ChannelsConfig::default(),
            HttpProxyPolicy::default(),
            ChannelSupervisionPolicy::default(),
        )
        .await
        .expect("confidential supervised start contract");
    lease
        .object()
        .start_outbound_channel(
            "discord".into(),
            ChannelsConfig::default(),
            HttpProxyPolicy::default(),
        )
        .await
        .expect("confidential outbound start contract");
    assert_eq!(
        lease
            .object()
            .prepare_channel_control(ChannelControlRequest::RuntimeLimits {
                timeout_secs: 47,
                listener_count: 2,
            })
            .await
            .expect("confidential channel control response"),
        ChannelControlResult::RuntimeLimits {
            timeout_secs: 47,
            max_in_flight: 8,
        }
    );
    assert_eq!(
        lease
            .object()
            .execute_progressive(ProgressiveCommand::Close {
                handle: "progressive-fixture".into(),
            })
            .await
            .expect("confidential progressive result"),
        ProgressiveResult::Closed
    );
    let prepared = lease
        .object()
        .prepare_outbound_intent(
            "discord".into(),
            ChannelMessagePreparation::Legacy(serde_json::json!({"text":"fixture"})),
        )
        .await
        .expect("confidential prepared intent");
    assert!(prepared.legacy_message.is_null());
    assert!(matches!(
        lease
            .object()
            .channel_interaction(ChannelInteractionRequest {
                channel: "discord".into(),
                interaction: ChannelInteraction::Describe,
            })
            .await
            .expect("confidential channel interaction"),
        ChannelInteractionResult::Completed
    ));
    let relay = lease
        .object()
        .reserve_relay()
        .await
        .expect("ordinary relay reservation");
    assert_eq!(relay, RelayReservation("fixture-relay".into()));
    assert_eq!(
        lease
            .object()
            .start_relay(
                relay.clone(),
                tinychannels_bus::config::RelayRuntimeConfig::default()
            )
            .await
            .expect("confidential relay start descriptor")
            .platform,
        ""
    );
    lease
        .object()
        .stop_relay(relay.clone())
        .await
        .expect("ordinary relay stop");
    lease
        .object()
        .send_channel_intent("discord".into(), ChannelOutboundIntent::default())
        .await
        .expect("ordinary channel intent");
    lease
        .object()
        .send_relay_intent(relay, ChannelOutboundIntent::default())
        .await
        .expect("ordinary relay intent");
    lease.object().shutdown().await.expect("ordinary shutdown");

    assert!(
        lease.close().await.is_ok(),
        "decoded STOP unit reply is ACK"
    );
    let callback = fixture
        .runtime
        .connection()
        .proxy(
            tinychannels_bus::HOST_BUS_NAME,
            RUNTIME_CALLBACK_PATH,
            tinychannels_bus::HOST_BUS_NAME,
        )
        .expect("callback probe proxy");
    assert!(
        callback
            .call::<String>("FixtureCallback", ())
            .await
            .is_err(),
        "only STOP ACK withdraws the callback registration"
    );

    // A's native connection closes on this fixture-only control call. Its
    // already-held root and object routes must fail while ordinary B remains
    // available. The failure path must not fall through to B.
    let _ = control.call::<()>("FixtureDisconnect", ()).await;
    assert!(control.call::<bool>("FixtureReady", ()).await.is_err());
    assert!(
        lease
            .object()
            .channel_status("discord".into())
            .await
            .is_err()
    );
    let after_disconnect: bool = fixture
        .control
        .call("tinychannels", "FixtureReady", ())
        .await
        .expect("ordinary B remains available after A disconnects");
    assert!(after_disconnect);
    assert_eq!(
        replacement_calls.load(Ordering::SeqCst),
        b_calls_before_lease + 1,
        "the post-disconnect ordinary check reached B, while A's bound object did not"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn dispatch_policy_uses_only_the_explicit_runtime_start_method() {
    let fixture = support::runtime_fixture().await;
    let driver = fixture
        .client
        .runtime_driver_with_dispatch(ChannelRuntimeDispatchPolicy { listener_count: 2 })
        .await
        .expect("bind with an explicit host-selected dispatch policy");
    let lease = fixture
        .runtime
        .lease(
            driver,
            |_, _| Ok(FixtureCallback),
            RetryPolicy::new(Duration::from_secs(2), vec![]).expect("valid stop policy"),
            |_| {},
        )
        .await
        .expect("StartRuntimeWithDispatch exact policy reply");
    assert!(lease.close().await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn actual_native_module_composes_provider_free_runtime_and_callback_registration() {
    let fixture = support::native_module_runtime_fixture().await;
    let driver = fixture
        .client
        .runtime_driver()
        .await
        .expect("strictly bind the locally built TinyChannels module artifact");
    let callback_path = Arc::new(std::sync::Mutex::new(None::<String>));
    let captured_callback_path = Arc::clone(&callback_path);
    let lease = fixture
        .runtime
        .lease(
            driver,
            move |reservation: &ChannelRuntimeReservation, _| {
                *captured_callback_path
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some(reservation.host_object_path.clone());
                Ok(FixtureCallback)
            },
            RetryPolicy::new(Duration::from_secs(2), vec![]).expect("valid stop policy"),
            |_| {},
        )
        .await
        .expect("real module reserves, binds a private object, exports callback and starts");

    // No providers are configured; this exercises the real module's typed
    // object contract without contacting an external provider or service.
    let _configured = lease
        .object()
        .describe_configured_channels(ChannelsConfig::default(), HttpProxyPolicy::default())
        .await
        .expect("real module provider description");
    let path = callback_path
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .expect("callback factory received the actual reservation path");
    assert_eq!(
        fixture
            .runtime
            .connection()
            .proxy(
                tinychannels_bus::HOST_BUS_NAME,
                &path,
                tinychannels_bus::HOST_BUS_NAME,
            )
            .expect("registered host callback proxy")
            .call::<String>("FixtureCallback", ())
            .await
            .expect("callback is reachable at the actual returned path"),
        "callback-live"
    );
    assert!(lease.close().await.is_ok(), "real STOP unit reply is ACK");
    assert!(
        fixture
            .runtime
            .connection()
            .proxy(
                tinychannels_bus::HOST_BUS_NAME,
                &path,
                tinychannels_bus::HOST_BUS_NAME,
            )
            .expect("callback proxy after STOP")
            .call::<String>("FixtureCallback", ())
            .await
            .is_err(),
        "real module STOP ACK releases the callback registration"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn opaque_start_and_stop_errors_remain_unacknowledged_and_keep_callback_live() {
    let fixture = support::runtime_fixture().await;
    let driver = fixture
        .client
        .runtime_driver()
        .await
        .expect("strictly bind the native fixture");
    let bound = fixture
        .control
        .bind_attested("tinychannels")
        .await
        .expect("hold fixture controls on the same native peer");
    bound
        .call::<()>("FixtureFailStart", ())
        .await
        .expect("configure an opaque start failure");
    let activation = fixture
        .runtime
        .lease(
            driver,
            |_, _| Ok(FixtureCallback),
            RetryPolicy::new(Duration::from_millis(100), vec![])
                .expect("valid start fixture policy"),
            |_| {},
        )
        .await;
    assert!(activation.is_err(), "opaque START failure is not success");
    let stop_calls: usize = bound
        .call("FixtureStopCallCount", ())
        .await
        .expect("read stop count after unacknowledged start");
    assert_eq!(
        stop_calls, 0,
        "unacknowledged START is never rolled back by STOP"
    );
    assert_eq!(
        driverless_callback_probe(fixture.runtime)
            .await
            .expect("unacknowledged START retains the callback"),
        "callback-live"
    );

    // Use a separate staged module instance for STOP uncertainty so the
    // intentionally retained START callback does not share test state.
    let fixture = support::runtime_fixture().await;
    let driver = fixture
        .client
        .runtime_driver()
        .await
        .expect("strictly bind the lease fixture");
    let bound = fixture
        .control
        .bind_attested("tinychannels")
        .await
        .expect("hold lease fixture controls on the same peer");
    let lease = fixture
        .runtime
        .lease(
            driver,
            |_, _| Ok(FixtureCallback),
            RetryPolicy::new(Duration::from_millis(100), vec![Duration::from_millis(1)])
                .expect("valid finite retry policy"),
            |_| {},
        )
        .await
        .expect("start the runtime before testing uncertain STOP");
    bound
        .call::<()>("FixtureFailStop", ())
        .await
        .expect("configure the opaque cleanup-pending stop failure");

    assert!(lease.close().await.is_err());
    assert!(lease.close().await.is_err());
    let stop_calls: usize = bound
        .call("FixtureStopCallCount", ())
        .await
        .expect("read the fixture's stop count");
    assert_eq!(stop_calls, 1, "uncertain STOP is never retried");
    assert_eq!(
        driverless_callback_probe(fixture.runtime)
            .await
            .expect("unacknowledged STOP retains the exact callback"),
        "callback-live"
    );
}

async fn driverless_callback_probe(
    runtime: &'static tinybus::module::ModuleRuntime,
) -> std::result::Result<String, ModuleCallError> {
    runtime
        .connection()
        .proxy(
            tinychannels_bus::HOST_BUS_NAME,
            RUNTIME_CALLBACK_PATH,
            tinychannels_bus::HOST_BUS_NAME,
        )
        .map_err(|_| tinybus::ModuleCallError::IncompatibleContract)?
        .call("FixtureCallback", ())
        .await
}
