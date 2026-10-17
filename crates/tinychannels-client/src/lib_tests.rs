//! Product-facing client calls over an in-memory TinyBus.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use serde::Deserialize;
use serde_json::{Value, json};
use tinybus::{
    Connection, InterfaceName, MemberName, ObjectPath,
    broker::Broker,
    module::{
        AccessFuture, FailureReport, FailureReporter, FailureSink, ModuleAccess, ModuleCallError,
        ModuleClient, ProxyFailure, ReleaseRecord,
    },
    service::Interface,
    transport::memory::MemoryBus,
};

use super::*;

const MODULE_ID: &str = "tinychannels";
type Calls = Arc<Mutex<Vec<(String, Value)>>>;
type Responses = Arc<Mutex<HashMap<String, Value>>>;
type Reports = Arc<Mutex<Vec<(String, String, String)>>>;

static LINKED_FIXTURES: OnceLock<Mutex<HashMap<String, (Calls, Responses)>>> = OnceLock::new();

static RELEASE: ReleaseRecord = ReleaseRecord {
    id: MODULE_ID,
    bus_name: tinychannels_bus::BUS_NAME,
    object_path: tinychannels_bus::OBJECT_PATH,
    version: "0.1.36",
    release_url: "https://example.invalid/tinychannels",
    assets: &[],
};

#[derive(Default)]
struct RecordingFailures(Reports);

impl FailureSink for RecordingFailures {
    fn capture(&self, report: &FailureReport<'_>) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((
                report.module.to_owned(),
                report.version.to_owned(),
                report.reason.code().to_owned(),
            ));
    }
}

struct FixtureAccess {
    reporter: FailureReporter,
}

impl ModuleAccess for FixtureAccess {
    fn find(&self, id: &str) -> Option<&'static ReleaseRecord> {
        (id == MODULE_ID).then_some(&RELEASE)
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

    fn proxy<'a>(
        &'a self,
        _bus_name: &'a str,
        _object_path: &'a str,
    ) -> AccessFuture<'a, Result<tinybus::Proxy, ProxyFailure>> {
        Box::pin(async { Err(ProxyFailure::Transport) })
    }
}

struct RecordingModule {
    calls: Calls,
    responses: Responses,
}

#[derive(Deserialize)]
struct LinkedFixtureConfig {
    fixture_id: String,
}

async fn linked_fixture_setup(
    connection: Connection,
    config: LinkedFixtureConfig,
) -> tinybus::Result<()> {
    let (calls, responses) = LINKED_FIXTURES
        .get()
        .and_then(|fixtures| fixtures.lock().ok())
        .and_then(|fixtures| fixtures.get(&config.fixture_id).cloned())
        .ok_or_else(|| tinybus::Error::failed("linked fixture is unavailable"))?;
    connection
        .serve_at(
            ObjectPath::new(tinychannels_bus::OBJECT_PATH)?,
            RecordingModule { calls, responses },
        )
        .await?;
    connection.request_name(tinychannels_bus::BUS_NAME).await
}

tinybus_module::module_export_static! {
    setup = linked_fixture_setup,
    config = LinkedFixtureConfig,
    worker_threads = 1,
    provides = [tinychannels_bus::BUS_NAME],
    methods = [],
    signals = [],
    requires = [],
    optional = [],
    lazy = false,
}

#[async_trait::async_trait]
impl Interface for RecordingModule {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(tinychannels_bus::BUS_NAME)
            .unwrap_or_else(|_| std::process::abort())
    }

    fn members(&self) -> Vec<MemberName> {
        tinychannels_bus::METHODS
            .iter()
            .filter_map(|member| MemberName::new(*member).ok())
            .collect()
    }

    async fn call(&self, member: &MemberName, args: Value) -> tinybus::Result<Value> {
        let member = member.as_str();
        self.calls
            .lock()
            .map_err(|_| tinybus::Error::failed("fixture call log is poisoned"))?
            .push((member.to_owned(), args.clone()));
        let key = if member == tinychannels_bus::methods::CHANNEL_CONTROLLER {
            args.get(0)
                .and_then(|request| request.get("operation"))
                .and_then(Value::as_str)
                .map(|operation| format!("{member}.{operation}"))
                .unwrap_or_else(|| member.to_owned())
        } else {
            member.to_owned()
        };
        let override_reply = self
            .responses
            .lock()
            .map_err(|_| tinybus::Error::failed("fixture response map is poisoned"))?
            .get(&key)
            .cloned();
        Ok(override_reply.unwrap_or_else(|| default_reply(&key)))
    }
}

fn default_reply(key: &str) -> Value {
    match key {
        "ChannelController.discord_list_guilds" => json!({
            "result": "discord_guilds",
            "value": {"raw": [{"id":"guild-fixture", "name":"Fixture guild", "icon":null}]}
        }),
        "ChannelController.discord_list_channels" => json!({
            "result": "discord_channels",
            "value": {"raw": [{"id":"channel-fixture", "name":"fixture", "type":0, "position":3, "parent_id":"category-fixture"}]}
        }),
        "ChannelController.discord_check_permissions" => json!({
            "result": "discord_permissions",
            "value": {"raw": {"can_view_channel":true,"can_send_messages":false,"can_read_message_history":true,"missing_permissions":["SEND_MESSAGES"]}}
        }),
        "ChannelController.prepare_connect" => json!({
            "result": "prepared_connect",
            "value": {"result":{"status":"connected","restart_required":false},"config_update":null}
        }),
        tinychannels_bus::methods::PREPARE_OUTBOUND_INTENT => json!({
            "intent": {
                "idempotencyKey":"prepared-fixture-key",
                "channelId":"discord",
                "conversationId":"conversation-fixture",
                "replyToId":null,
                "threadId":null,
                "durability":"best_effort",
                "payload":{"kind":"text","text":"fixture message"}
            },
            "legacy_message":{"content":"fixture message","recipient":"conversation-fixture"}
        }),
        tinychannels_bus::methods::PREPARE_CHANNEL_IDENTITY => json!({
            "thread_id":"thread-fixture", "client_id":"client-fixture"
        }),
        tinychannels_bus::methods::PREPARE_CHANNEL_INBOUND => json!({
            "message":{"id":"message-fixture","sender":"sender-fixture","reply_target":"reply-fixture","content":"hello fixture","channel":"discord","timestamp":7},
            "envelope":{
                "channel":{"id":"discord","name":"discord"},
                "messageId":"message-fixture",
                "conversation":{"id":"conversation-fixture"},
                "sender":{"id":"sender-fixture"},
                "text":"hello fixture",
                "access":{},
                "media":[],
                "raw":null
            },
            "history_key":"history-fixture",
            "thread_id":"thread-fixture",
            "client_id":"client-fixture",
            "session_key":"session-fixture"
        }),
        tinychannels_bus::methods::CHECK_CHANNEL_HEALTH => json!([
            {"channel":"discord","state":"healthy"},
            {"channel":"telegram","state":"timeout"}
        ]),
        _ => Value::Null,
    }
}

struct Fixture {
    _host: tinybus::module::ModuleHost,
    _caller: Connection,
    client: TinyChannelsClient,
    calls: Calls,
    reports: Reports,
}

async fn fixture(
    responses: HashMap<String, Value>,
) -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(1);
    let fixture_id = format!("tinychannels-client-{}", NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(responses));
    LINKED_FIXTURES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| std::io::Error::other("fixture registry is poisoned"))?
        .insert(fixture_id.clone(), (Arc::clone(&calls), responses));

    let bus = MemoryBus::new();
    let broker = Broker::new();
    broker.clone().spawn(bus.clone());
    let host = tinybus::module::ModuleHost::new(broker);
    host.attach_linked_with_config(
        linked_module()?,
        json!({"fixture_id": fixture_id}),
    )?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if host
                .list()
                .iter()
                .any(|module| matches!(module.state, tinybus::module::ModuleState::Ready))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;

    let caller = Connection::connect(bus.connect().await?).await?;
    let proxy = caller.proxy(
        tinychannels_bus::BUS_NAME,
        tinychannels_bus::OBJECT_PATH,
        tinychannels_bus::BUS_NAME,
    )?;
    let reports = Arc::new(Mutex::new(Vec::new()));
    let reporter = FailureReporter::new(Arc::new(RecordingFailures(Arc::clone(&reports))));
    let module = ModuleClient::new(FixtureAccess { reporter }).with_fixture(proxy);
    Ok(Fixture {
        _host: host,
        _caller: caller,
        client: TinyChannelsClient::new(module),
        calls,
        reports,
    })
}

fn expected(member: &str, args: impl serde::Serialize) -> (String, Value) {
    (member.to_owned(), serde_json::to_value(args).expect("test wire args serialize"))
}

#[tokio::test]
async fn typed_operations_preserve_contract_members_positional_args_and_decoded_results()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture(HashMap::new()).await?;
    let config = tinychannels_bus::ChannelsConfig::default();
    let proxy = tinychannels_bus::HttpProxyPolicy {
        https_proxy: Some("http://proxy.fixture.invalid:3128".into()),
        no_proxy: vec!["localhost".into()],
        service_selectors: vec!["channel.discord".into()],
        ..Default::default()
    };

    let guilds = fixture
        .client
        .discord_list_guilds(config.clone(), "fixture-discord-token".into(), proxy.clone())
        .await?;
    assert_eq!(guilds[0].id, "guild-fixture");
    assert_eq!(guilds[0].icon, None);

    let channels = fixture
        .client
        .discord_list_channels(
            config.clone(),
            "guild-fixture".into(),
            "fixture-discord-token".into(),
            proxy.clone(),
        )
        .await?;
    assert_eq!(channels[0].channel_type, 0);
    assert_eq!(channels[0].position, 3);
    assert_eq!(channels[0].parent_id.as_deref(), Some("category-fixture"));

    let permissions = fixture
        .client
        .discord_check_permissions(
            config.clone(),
            "guild-fixture".into(),
            "channel-fixture".into(),
            "fixture-discord-token".into(),
            proxy.clone(),
        )
        .await?;
    assert!(permissions.can_view_channel);
    assert!(!permissions.can_send_messages);
    assert_eq!(permissions.missing_permissions, ["SEND_MESSAGES"]);

    let request = tinychannels_bus::ChannelControllerRequest::PrepareConnect {
        config: config.clone(),
        channel: "discord".into(),
        auth_mode: tinychannels_bus::ChannelAuthMode::BotToken,
        credentials: json!({"bot_token":"fixture-only-token"}),
        proxy: proxy.clone(),
    };
    let prepared = fixture.client.prepare_connection(request.clone()).await?;
    assert_eq!(prepared.result.status, "connected");

    let message = json!({"content":"fixture message","recipient":"conversation-fixture"});
    let intent = fixture
        .client
        .prepare_message("discord", message.clone())
        .await?;
    assert_eq!(intent.intent.idempotency_key, "prepared-fixture-key");

    let identity = tinychannels_bus::module_inbound::PrepareChannelIdentityRequest {
        channel: "discord".into(),
        sender: Some("fictional-sender".into()),
        reply_target: Some("fictional-reply-target".into()),
        thread_ts: Some("fixture-thread-ts".into()),
    };
    let prepared_identity = fixture.client.prepare_identity(identity.clone()).await?;
    assert_eq!(prepared_identity.thread_id, "thread-fixture");

    let inbound = tinychannels_bus::module_inbound::PrepareChannelInboundRequest {
        source: tinychannels_bus::module_inbound::ChannelInboundSource::Legacy {
            message: tinychannels_bus::ChannelMessage {
                id: "message-fixture".into(),
                sender: "fictional-sender".into(),
                reply_target: "fictional-reply-target".into(),
                content: "fixture inbound".into(),
                channel: "discord".into(),
                timestamp: 7,
                ..Default::default()
            },
            envelope: None,
        },
        namespace: "fixture-namespace".into(),
        session_policy: Default::default(),
    };
    let prepared_inbound = fixture.client.prepare_inbound(inbound.clone()).await?;
    assert_eq!(prepared_inbound.session_key, "session-fixture");

    let health = fixture
        .client
        .check_health(config.clone(), proxy.clone(), 4312)
        .await?;
    assert_eq!(health[0].channel, "discord");
    assert_eq!(health[0].state, tinychannels_bus::ChannelHealthState::Healthy);
    assert_eq!(health[1].state, tinychannels_bus::ChannelHealthState::Timeout);

    let actual = fixture
        .calls
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    assert_eq!(
        actual,
        [
            expected(
                tinychannels_bus::methods::CHANNEL_CONTROLLER,
                (tinychannels_bus::ChannelControllerRequest::DiscordListGuilds {
                    config: config.clone(),
                    bot_token: "fixture-discord-token".into(),
                    proxy: proxy.clone(),
                },),
            ),
            expected(
                tinychannels_bus::methods::CHANNEL_CONTROLLER,
                (tinychannels_bus::ChannelControllerRequest::DiscordListChannels {
                    config: config.clone(),
                    guild_id: "guild-fixture".into(),
                    bot_token: "fixture-discord-token".into(),
                    proxy: proxy.clone(),
                },),
            ),
            expected(
                tinychannels_bus::methods::CHANNEL_CONTROLLER,
                (tinychannels_bus::ChannelControllerRequest::DiscordCheckPermissions {
                    config: config.clone(),
                    guild_id: "guild-fixture".into(),
                    channel_id: "channel-fixture".into(),
                    bot_token: "fixture-discord-token".into(),
                    proxy: proxy.clone(),
                },),
            ),
            expected(tinychannels_bus::methods::CHANNEL_CONTROLLER, (request,)),
            expected(
                tinychannels_bus::methods::PREPARE_OUTBOUND_INTENT,
                ("discord", tinychannels_bus::module_runtime::ChannelMessagePreparation::Legacy(message)),
            ),
            expected(tinychannels_bus::methods::PREPARE_CHANNEL_IDENTITY, (identity,)),
            expected(tinychannels_bus::methods::PREPARE_CHANNEL_INBOUND, (inbound,)),
            expected(
                tinychannels_bus::methods::CHECK_CHANNEL_HEALTH,
                (config, proxy, 4312_u64),
            ),
        ]
    );
    assert!(fixture
        .reports
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn malformed_discord_projections_are_reported_as_closed_sanitized_faults()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    for reply in [
        json!({"result":"discord_channels","value":{"raw":[]}}),
        json!({"result":"discord_guilds","value":{}}),
        json!({"result":"discord_guilds","value":{"raw":[{"id":"private-fixture-detail"}]}}),
    ] {
        let fixture = fixture(HashMap::from([(
            "ChannelController.discord_list_guilds".into(),
            reply,
        )]))
        .await?;
        let error = fixture
            .client
            .discord_list_guilds(
                tinychannels_bus::ChannelsConfig::default(),
                "fixture-secret-token".into(),
                Default::default(),
            )
            .await
            .expect_err("malformed wire reply is terminal");
        assert_eq!(error, ModuleCallError::ModuleFault);
        assert!(!error.to_string().contains("private-fixture-detail"));
        let reports = fixture
            .reports
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(reports.as_slice(), [(MODULE_ID.into(), "0.1.36".into(), "module_fault".into())]);
    }
    Ok(())
}

#[tokio::test]
async fn confidential_discord_calls_do_not_fall_back_to_an_unattested_member()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let bus = MemoryBus::new();
    Broker::new().spawn(bus.clone());
    let service = Connection::connect(bus.connect().await?).await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    service
        .serve_at(
            ObjectPath::new(tinychannels_bus::OBJECT_PATH)?,
            RecordingModule {
                calls: Arc::clone(&calls),
                responses: Arc::new(Mutex::new(HashMap::new())),
            },
        )
        .await?;
    service.request_name(tinychannels_bus::BUS_NAME).await?;
    let caller = Connection::connect(bus.connect().await?).await?;
    let proxy = caller.proxy(
        tinychannels_bus::BUS_NAME,
        tinychannels_bus::OBJECT_PATH,
        tinychannels_bus::BUS_NAME,
    )?;
    let reporter = FailureReporter::new(Arc::new(RecordingFailures(Arc::new(Mutex::new(Vec::new())))));
    let client = TinyChannelsClient::new(
        ModuleClient::new(FixtureAccess { reporter }).with_fixture(proxy),
    );
    let error = client
        .discord_list_guilds(
            tinychannels_bus::ChannelsConfig::default(),
            "fixture-secret-token".into(),
            Default::default(),
        )
        .await
        .expect_err("confidential request must require module attestation");
    assert_eq!(error, ModuleCallError::TransportFailed);
    assert!(calls
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_empty());
    Ok(())
}
