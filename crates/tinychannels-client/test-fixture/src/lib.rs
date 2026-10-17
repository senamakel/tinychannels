//! Native test-only module for checking the public TinyChannels client routes.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tinybus::{Connection, Error, Interface, InterfaceName, MemberName, ObjectPath, Result};
use tinychannels_bus::{
    ChannelControllerRequest, ChannelControllerResult, ChannelsConfig, HttpProxyPolicy,
    controllers::{DiscordChannelListResult, DiscordGuildListResult, DiscordPermissionCheckResult},
    methods,
};

const BUS_NAME: &str = tinychannels_bus::BUS_NAME;
const OBJECT_PATH: &str = tinychannels_bus::OBJECT_PATH;
const TOKEN: &str = "fixture-call-secret";
const CONFIG_TOKEN: &str = "fixture-config-secret";
const INVALID_INPUT: &str = "ai.tinyhumans.tinychannels.Error.InvalidInput";

#[derive(Deserialize)]
struct FixtureConfig {}

struct FixtureService {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl Interface for FixtureService {
    fn name(&self) -> InterfaceName {
        InterfaceName::new(BUS_NAME).expect("TinyChannels bus name is valid")
    }

    fn members(&self) -> Vec<MemberName> {
        [
            methods::CHANNEL_CONTROLLER,
            methods::CHECK_CHANNEL_HEALTH,
            "FixtureCallCount",
        ]
        .into_iter()
        .map(|method| MemberName::new(method).expect("TinyChannels member is valid"))
        .collect()
    }

    fn requires_confidential(&self, member: &MemberName) -> bool {
        matches!(
            member.as_str(),
            methods::CHANNEL_CONTROLLER | methods::CHECK_CHANNEL_HEALTH
        )
    }

    async fn call(&self, member: &MemberName, args: Value) -> Result<Value> {
        if member.as_str() == "FixtureCallCount" {
            return Ok(json!(self.calls.load(Ordering::SeqCst)));
        }
        self.calls.fetch_add(1, Ordering::SeqCst);
        match member.as_str() {
            methods::CHANNEL_CONTROLLER => self.channel_controller(args),
            methods::CHECK_CHANNEL_HEALTH => self.check_health(args),
            _ => Err(Error::failed("unknown test fixture method")),
        }
    }
}

impl FixtureService {
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
        .serve_at(ObjectPath::new(OBJECT_PATH)?, FixtureService { calls })
        .await?;
    connection.request_name(BUS_NAME).await
}

tinybus_module::module_export! {
    setup = setup,
    config = FixtureConfig,
    worker_threads = 1,
    provides = ["ai.tinyhumans.tinychannels.Channels"],
    methods = ["ChannelController", "CheckChannelHealth", "FixtureCallCount"],
    signals = [],
    requires = [],
    optional = [],
    lazy = false,
}
