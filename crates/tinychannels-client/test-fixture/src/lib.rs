//! Native test-only module for checking the public TinyChannels client routes.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tinybus::{Connection, Error, Interface, InterfaceName, MemberName, ObjectPath, Result};
use tinychannels_bus::{
    ChannelControllerRequest, ChannelControllerResult, ChannelInteractionRequest,
    ChannelInteractionResult, ChannelOutboundIntent, ChannelRuntimeDispatchPolicy,
    ChannelRuntimeReservation, ChannelSupervisionPolicy, ChannelsConfig, HttpProxyPolicy,
    controllers::{
        ChannelSendMessageResult, DiscordChannelListResult, DiscordGuildListResult,
        DiscordPermissionCheckResult,
    },
    methods, module_control::{ChannelControlRequest, ChannelControlResult},
    module_progressive::{ProgressiveCommand, ProgressiveResult},
    module_relay::RelayReservation,
};

const BUS_NAME: &str = tinychannels_bus::BUS_NAME;
const OBJECT_PATH: &str = tinychannels_bus::OBJECT_PATH;
const TOKEN: &str = "fixture-call-secret";
const CONFIG_TOKEN: &str = "fixture-config-secret";
const INVALID_INPUT: &str = "ai.tinyhumans.tinychannels.Error.InvalidInput";
const RUNTIME_HANDLE: &str = "0123456789abcdef0123456789abcdef";
const RUNTIME_OBJECT_PATH: &str =
    "/ai/tinyhumans/tinychannels/Channels/runtime_0123456789abcdef0123456789abcdef";
const RUNTIME_CALLBACK_PATH: &str =
    "/ai/tinyhumans/tinychannels/ChannelsHost/runtime_0123456789abcdef0123456789abcdef";

#[derive(Deserialize)]
struct FixtureConfig {}

struct FixtureService {
    connection: Connection,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl Interface for FixtureService {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(BUS_NAME).expect("TinyChannels bus name is valid")
    }

    fn members(&self) -> Vec<MemberName> {
        tinychannels_bus::METHODS
        .into_iter()
        .chain([
            "FixtureCallCount",
            "FixtureReady",
            "FixtureReleaseAlias",
            "FixtureDisconnect",
        ])
        .into_iter()
        .map(|method| MemberName::new(method).expect("TinyChannels member is valid"))
        .collect()
    }

    fn requires_confidential(&self, member: &MemberName) -> bool {
        matches!(
            member.as_str(),
            methods::CHANNEL_CONTROLLER
                | methods::CHECK_CHANNEL_HEALTH
                | methods::RESERVE_RUNTIME
                | methods::START_RUNTIME
                | methods::START_RUNTIME_WITH_DISPATCH
                | methods::STOP_RUNTIME
                | methods::DESCRIBE_CONFIGURED_CHANNELS
                | methods::START_CHANNEL_SUPERVISED
                | methods::START_OUTBOUND_CHANNEL
                | methods::PREPARE_CHANNEL_CONTROL
                | methods::EXECUTE_PROGRESSIVE
                | methods::PREPARE_OUTBOUND_INTENT
                | methods::CHANNEL_INTERACTION
                | methods::START_RELAY
        )
    }

    async fn call(&self, member: &MemberName, args: Value) -> Result<Value> {
        if member.as_str() == "FixtureCallCount" {
            return Ok(json!(self.calls.load(Ordering::SeqCst)));
        }
        self.calls.fetch_add(1, Ordering::SeqCst);
        match member.as_str() {
            "FixtureReady" => Ok(json!(true)),
            "FixtureReleaseAlias" => {
                self.connection.release_name(BUS_NAME).await?;
                Ok(Value::Null)
            }
            "FixtureDisconnect" => {
                self.connection.close().await?;
                Ok(Value::Null)
            }
            methods::RESERVE_RUNTIME => {
                arguments(&args, 0)?;
                self.reserve_runtime().await
            }
            methods::START_RUNTIME => self.start_runtime(args, false).await,
            methods::START_RUNTIME_WITH_DISPATCH => self.start_runtime(args, true).await,
            methods::STOP_RUNTIME => {
                if first_string(&args)? != RUNTIME_HANDLE {
                    return Err(Error::failed("runtime stop handle mismatch"));
                }
                Ok(Value::Null)
            }
            methods::CHANNEL_CONTROLLER => self.channel_controller(args),
            methods::CHECK_CHANNEL_HEALTH => self.check_health(args),
            _ => Err(Error::failed("unknown test fixture method")),
        }
    }
}

impl FixtureService {
    async fn reserve_runtime(&self) -> Result<Value> {
        let reservation = ChannelRuntimeReservation {
            handle: RUNTIME_HANDLE.into(),
            object_path: RUNTIME_OBJECT_PATH.into(),
            host_object_path: RUNTIME_CALLBACK_PATH.into(),
        };
        self.connection
            .serve_at(
                ObjectPath::new(RUNTIME_OBJECT_PATH)?,
                FixtureObject {
                    calls: Arc::clone(&self.calls),
                },
            )
            .await?;
        serde_json::to_value(reservation).map_err(|_| Error::failed("fixture serialization failed"))
    }

    async fn start_runtime(&self, args: Value, with_dispatch: bool) -> Result<Value> {
        let values = arguments(&args, if with_dispatch { 2 } else { 1 })?;
        if values.first().and_then(Value::as_str) != Some(RUNTIME_HANDLE) {
            return Err(Error::failed("runtime start handle mismatch"));
        }
        if with_dispatch {
            let policy: ChannelRuntimeDispatchPolicy = decode(values, 1)?;
            if policy.listener_count != 2 {
                return Err(Error::failed("runtime dispatch policy mismatch"));
            }
        }
        let callback = self
            .connection
            .proxy(
                tinychannels_bus::HOST_BUS_NAME,
                RUNTIME_CALLBACK_PATH,
                tinychannels_bus::HOST_BUS_NAME,
            )?
            .call::<String>("FixtureCallback", ())
            .await?;
        if callback != "callback-live" {
            return Err(Error::failed("runtime callback was not registered"));
        }
        Ok(Value::Null)
    }

    fn channel_controller(&self, args: Value) -> Result<Value> {
        let request = args
            .get(0)
            .cloned()
            .and_then(|value| serde_json::from_value::<ChannelControllerRequest>(value).ok())
            .ok_or_else(|| Error::failed("invalid test fixture request"))?;
        match request {
            ChannelControllerRequest::DiscordListGuilds {
                config,
                bot_token,
                proxy,
            } => {
                validate_request(&config, &bot_token, &proxy, None, None)?;
                if proxy.service_selectors == ["test:invalid-input"] {
                    return Err(invalid_input());
                }
                let result = match scenario(&proxy) {
                    "test:wrong-variant" => {
                        ChannelControllerResult::DiscordChannels(DiscordChannelListResult {
                            raw: Some(json!([])),
                            ..Default::default()
                        })
                    }
                    "test:missing-raw" => {
                        ChannelControllerResult::DiscordGuilds(DiscordGuildListResult::default())
                    }
                    "test:malformed-payload" => {
                        ChannelControllerResult::DiscordGuilds(DiscordGuildListResult {
                            raw: Some(json!({"private":"fixture"})),
                            ..Default::default()
                        })
                    }
                    _ => ChannelControllerResult::DiscordGuilds(DiscordGuildListResult {
                        raw: Some(json!([{"id":"guild-1","name":"Fixture guild","icon":null}])),
                        ..Default::default()
                    }),
                };
                serde_json::to_value(result)
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            ChannelControllerRequest::DiscordListChannels {
                config,
                guild_id,
                bot_token,
                proxy,
            } => {
                validate_request(&config, &bot_token, &proxy, Some(&guild_id), None)?;
                let result = match scenario(&proxy) {
                    "test:wrong-variant" => {
                        ChannelControllerResult::DiscordGuilds(DiscordGuildListResult {
                            raw: Some(json!([])),
                            ..Default::default()
                        })
                    }
                    "test:missing-raw" => {
                        ChannelControllerResult::DiscordChannels(DiscordChannelListResult::default())
                    }
                    _ => ChannelControllerResult::DiscordChannels(DiscordChannelListResult {
                        raw: Some(
                            json!([{"id":"channel-1","name":"general","type":0,"position":11,"parent_id":"category-1"}]),
                        ),
                        ..Default::default()
                    }),
                };
                serde_json::to_value(result)
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            ChannelControllerRequest::DiscordCheckPermissions {
                config,
                guild_id,
                channel_id,
                bot_token,
                proxy,
            } => {
                validate_request(
                    &config,
                    &bot_token,
                    &proxy,
                    Some(&guild_id),
                    Some(&channel_id),
                )?;
                let result = match scenario(&proxy) {
                    "test:wrong-variant" => {
                        ChannelControllerResult::DiscordGuilds(DiscordGuildListResult {
                            raw: Some(json!([])),
                            ..Default::default()
                        })
                    }
                    "test:missing-raw" => ChannelControllerResult::DiscordPermissions(
                        DiscordPermissionCheckResult::default(),
                    ),
                    "test:malformed-payload" => {
                        ChannelControllerResult::DiscordPermissions(DiscordPermissionCheckResult {
                            raw: Some(json!(false)),
                            ..Default::default()
                        })
                    }
                    _ => {
                        ChannelControllerResult::DiscordPermissions(DiscordPermissionCheckResult {
                            raw: Some(
                                json!({"can_view_channel":true,"can_send_messages":false,"can_read_message_history":true,"missing_permissions":["SEND_MESSAGES"]}),
                            ),
                            ..Default::default()
                        })
                    }
                };
                serde_json::to_value(result)
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            _ => Err(Error::failed("unexpected controller operation")),
        }
    }

    fn check_health(&self, args: Value) -> Result<Value> {
        let Some(values) = args.as_array() else {
            return Err(Error::failed("invalid health fixture arguments"));
        };
        let config = values
            .first()
            .cloned()
            .and_then(|value| serde_json::from_value::<ChannelsConfig>(value).ok())
            .ok_or_else(|| Error::failed("invalid health config"))?;
        let proxy = values
            .get(1)
            .cloned()
            .and_then(|value| serde_json::from_value::<HttpProxyPolicy>(value).ok())
            .ok_or_else(|| Error::failed("invalid health proxy"))?;
        let timeout = values.get(2).and_then(Value::as_u64);
        validate_request(&config, TOKEN, &proxy, None, None)?;
        if timeout != Some(12_345) {
            return Err(invalid_input());
        }
        match scenario(&proxy) {
            "test:malformed-payload" => Ok(json!([{"channel":17,"state":"healthy"}])),
            "test:invalid-input" => Err(invalid_input()),
            _ => Ok(json!([
                {"channel":"discord","state":"healthy"},
                {"channel":"telegram","state":"timeout"}
            ])),
        }
    }
}

struct FixtureObject {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl Interface for FixtureObject {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(BUS_NAME).expect("TinyChannels bus name is valid")
    }

    fn members(&self) -> Vec<MemberName> {
        tinychannels_bus::METHODS
            .into_iter()
            .map(|method| MemberName::new(method).expect("TinyChannels member is valid"))
            .collect()
    }

    fn requires_confidential(&self, member: &MemberName) -> bool {
        matches!(
            member.as_str(),
            methods::DESCRIBE_CONFIGURED_CHANNELS
                | methods::START_CHANNEL_SUPERVISED
                | methods::START_OUTBOUND_CHANNEL
                | methods::PREPARE_CHANNEL_CONTROL
                | methods::EXECUTE_PROGRESSIVE
                | methods::PREPARE_OUTBOUND_INTENT
                | methods::CHANNEL_INTERACTION
                | methods::START_RELAY
        )
    }

    async fn call(&self, member: &MemberName, args: Value) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match member.as_str() {
            methods::DESCRIBE_CONFIGURED_CHANNELS => {
                let values = arguments(&args, 2)?;
                let _: ChannelsConfig = decode(values, 0)?;
                let _: HttpProxyPolicy = decode(values, 1)?;
                Ok(json!([]))
            }
            methods::START_CHANNEL_SUPERVISED => {
                let values = arguments(&args, 4)?;
                let _: String = decode(values, 0)?;
                let _: ChannelsConfig = decode(values, 1)?;
                let _: HttpProxyPolicy = decode(values, 2)?;
                let _: ChannelSupervisionPolicy = decode(values, 3)?;
                Ok(Value::Null)
            }
            methods::START_OUTBOUND_CHANNEL => {
                let values = arguments(&args, 3)?;
                let _: String = decode(values, 0)?;
                let _: ChannelsConfig = decode(values, 1)?;
                let _: HttpProxyPolicy = decode(values, 2)?;
                Ok(Value::Null)
            }
            methods::PREPARE_CHANNEL_CONTROL => {
                let values = arguments(&args, 1)?;
                let _: ChannelControlRequest = decode(values, 0)?;
                serde_json::to_value(ChannelControlResult::RuntimeLimits {
                    timeout_secs: 47,
                    max_in_flight: 8,
                })
                .map_err(|_| Error::failed("fixture serialization failed"))
            }
            methods::EXECUTE_PROGRESSIVE => {
                let values = arguments(&args, 1)?;
                let _: ProgressiveCommand = decode(values, 0)?;
                serde_json::to_value(ProgressiveResult::Closed)
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            methods::PREPARE_OUTBOUND_INTENT => {
                let values = arguments(&args, 2)?;
                let _: String = decode(values, 0)?;
                let _: tinychannels_bus::module_runtime::ChannelMessagePreparation =
                    decode(values, 1)?;
                serde_json::to_value(tinychannels_bus::module_runtime::PreparedChannelIntent {
                    intent: ChannelOutboundIntent::default(),
                    legacy_message: Value::Null,
                })
                .map_err(|_| Error::failed("fixture serialization failed"))
            }
            methods::CHANNEL_INTERACTION => {
                let values = arguments(&args, 1)?;
                let _: ChannelInteractionRequest = decode(values, 0)?;
                serde_json::to_value(ChannelInteractionResult::Completed)
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            methods::START_RELAY => {
                let values = arguments(&args, 2)?;
                let _: RelayReservation = decode(values, 0)?;
                let _: tinychannels_bus::config::RelayRuntimeConfig = decode(values, 1)?;
                serde_json::to_value(tinychannels_bus::relay::CapabilityDescriptor::default())
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            methods::RESERVE_RELAY => {
                arguments(&args, 0)?;
                serde_json::to_value(RelayReservation("fixture-relay".into()))
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            methods::STOP_RELAY => {
                let values = arguments(&args, 1)?;
                let _: RelayReservation = decode(values, 0)?;
                Ok(Value::Null)
            }
            methods::CHANNEL_STATUS => {
                let values = arguments(&args, 1)?;
                let _: String = decode(values, 0)?;
                Ok(json!("peer-a"))
            }
            methods::SEND_CHANNEL_INTENT => {
                let values = arguments(&args, 2)?;
                let _: String = decode(values, 0)?;
                let _: ChannelOutboundIntent = decode(values, 1)?;
                serde_json::to_value(ChannelSendMessageResult::default())
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            methods::SEND_RELAY_INTENT => {
                let values = arguments(&args, 2)?;
                let _: RelayReservation = decode(values, 0)?;
                let _: ChannelOutboundIntent = decode(values, 1)?;
                serde_json::to_value(ChannelSendMessageResult::default())
                    .map_err(|_| Error::failed("fixture serialization failed"))
            }
            methods::SHUTDOWN => {
                arguments(&args, 0)?;
                Ok(Value::Null)
            }
            _ => Err(Error::failed("unexpected TinyChannels object method")),
        }
    }
}

#[derive(Clone)]
struct FixtureCallback;

#[async_trait]
impl Interface for FixtureCallback {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(tinychannels_bus::HOST_BUS_NAME)
            .expect("TinyChannels callback bus name is valid")
    }

    fn members(&self) -> Vec<MemberName> {
        vec![MemberName::new("FixtureCallback").expect("valid fixture callback")]
    }

    async fn call(&self, _: &MemberName, _: Value) -> Result<Value> {
        Ok(json!("callback-live"))
    }
}

fn arguments(value: &Value, count: usize) -> Result<&[Value]> {
    let values = value
        .as_array()
        .ok_or_else(|| Error::failed("fixture arguments are not positional"))?;
    if values.len() != count {
        return Err(Error::failed("fixture argument count mismatch"));
    }
    Ok(values)
}

fn decode<T: DeserializeOwned>(values: &[Value], index: usize) -> Result<T> {
    values
        .get(index)
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .ok_or_else(|| Error::failed("fixture argument has the wrong type"))
}

fn first_string(args: &Value) -> Result<&str> {
    args.as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .ok_or_else(|| Error::failed("fixture runtime handle is missing"))
}

fn validate_request(
    config: &ChannelsConfig,
    bot_token: &str,
    proxy: &HttpProxyPolicy,
    guild_id: Option<&str>,
    channel_id: Option<&str>,
) -> Result<()> {
    if serde_json::to_value(config).ok() != serde_json::to_value(expected_config()).ok()
        || bot_token != TOKEN
        || guild_id.is_some_and(|actual| actual != "guild-1")
        || channel_id.is_some_and(|actual| actual != "channel-1")
        || !valid_proxy(proxy)
    {
        return Err(Error::MethodFailed {
            name: INVALID_INPUT.into(),
            message: "test fixture request did not match expected fields".into(),
        });
    }
    Ok(())
}

fn valid_proxy(proxy: &HttpProxyPolicy) -> bool {
    proxy.http_proxy.as_deref() == Some("http://proxy.example.test:8080")
        && proxy.https_proxy.as_deref() == Some("https://proxy.example.test:8443")
        && proxy.all_proxy.as_deref() == Some("socks5://proxy.example.test:1080")
        && proxy.no_proxy == ["localhost", ".internal"]
        && (proxy.service_selectors == ["channel.discord"] || scenario(proxy).starts_with("test:"))
}

fn scenario(proxy: &HttpProxyPolicy) -> &str {
    proxy.service_selectors.first().map_or("", String::as_str)
}

fn expected_config() -> ChannelsConfig {
    ChannelsConfig {
        cli: false,
        message_timeout_secs: 47,
        active_channel: Some("discord".into()),
        discord: Some(tinychannels_bus::config::DiscordConfig {
            bot_token: CONFIG_TOKEN.into(),
            guild_id: Some("config-guild".into()),
            channel_id: Some("config-channel".into()),
            allowed_users: vec!["member-1".into()],
            listen_to_bots: true,
            mention_only: true,
        }),
        ..Default::default()
    }
}

fn invalid_input() -> Error {
    Error::MethodFailed {
        name: INVALID_INPUT.into(),
        message: "request contained fixture-private-token; input is invalid".into(),
    }
}

async fn setup(connection: Connection, _config: FixtureConfig) -> Result<()> {
    let calls = Arc::new(AtomicUsize::new(0));
    connection
        .serve_at(
            ObjectPath::new(OBJECT_PATH)?,
            FixtureService {
                connection: connection.clone(),
                calls,
            },
        )
        .await?;
    connection.request_name(BUS_NAME).await
}

tinybus_module::module_export! {
    setup = setup,
    config = FixtureConfig,
    worker_threads = 1,
    provides = ["ai.tinyhumans.tinychannels.Channels"],
        methods = [
            "ChannelController", "CheckChannelHealth", "FixtureCallCount",
            "FixtureReady", "FixtureReleaseAlias", "FixtureDisconnect",
            "ReserveRuntime", "StartRuntime", "StartRuntimeWithDispatch", "StopRuntime",
            "DescribeConfiguredChannels", "StartChannelSupervised", "StartOutboundChannel",
            "PrepareChannelControl", "ExecuteProgressive", "PrepareOutboundIntent",
            "ChannelInteraction", "StartRelay", "ReserveRelay", "StopRelay", "ChannelStatus",
            "SendChannelIntent", "SendRelayIntent", "Shutdown",
        ],
    signals = [],
    requires = [],
    optional = [],
    lazy = false,
}
