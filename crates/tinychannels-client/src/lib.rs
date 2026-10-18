//! Typed host calls to confidential TinyChannels module operations.

use std::fmt;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use tinybus::{
    BusName, ObjectPath,
    module::{
        BoundModuleClient, ModuleCallError, ModuleCallFailure, ModuleClient, ModuleObjectClient,
        RuntimeLeaseDriver, StartOutcome, StopOutcome,
    },
};
use tinychannels_bus::{
    ChannelControllerRequest, ChannelControllerResult, ChannelInteractionRequest,
    ChannelInteractionResult, ChannelOutboundIntent, ChannelRuntimeDispatchPolicy,
    ChannelRuntimeReservation, ChannelSupervisionPolicy, ChannelsConfig, HttpProxyPolicy,
    controllers::{BotPermissionCheck, ChannelSendMessageResult, DiscordGuild, DiscordTextChannel},
    methods,
    module_control::{ChannelControlRequest, ChannelControlResult},
    module_progressive::{ProgressiveCommand, ProgressiveResult},
    module_relay::RelayReservation,
};

/// Typed host client for the TinyChannels module.
///
/// The host supplies the module registry and lifecycle through `ModuleClient`.
/// Calls that contain credentials or proxy policy use TinyBus's strict
/// attested confidential route and never retry as ordinary calls.
#[derive(Clone)]
pub struct TinyChannelsClient {
    module: ModuleClient,
}

impl TinyChannelsClient {
    /// Create a client over the host's TinyBus module access.
    #[must_use]
    pub fn new(module: ModuleClient) -> Self {
        Self { module }
    }

    /// Bind one attested TinyChannels peer for an identity-scoped runtime.
    ///
    /// The returned driver keeps its root calls and every later object call on
    /// the same unique peer, even if the well-known module name changes owners.
    /// OpenHuman supplies the callback implementation, retry policy, reporting,
    /// configuration and credentials when it passes this driver to
    /// `ModuleRuntime::lease`.
    pub async fn runtime_driver(&self) -> Result<ChannelRuntimeDriver, ChannelRuntimeError> {
        self.bind_runtime_driver(RuntimeStart::Serial).await
    }

    /// Bind a runtime whose inbound dispatch explicitly uses this host-selected
    /// concurrency policy. [`Self::runtime_driver`] retains serial dispatch.
    pub async fn runtime_driver_with_dispatch(
        &self,
        policy: ChannelRuntimeDispatchPolicy,
    ) -> Result<ChannelRuntimeDriver, ChannelRuntimeError> {
        self.bind_runtime_driver(RuntimeStart::Dispatch(policy))
            .await
    }

    async fn bind_runtime_driver(
        &self,
        start: RuntimeStart,
    ) -> Result<ChannelRuntimeDriver, ChannelRuntimeError> {
        let bound = self
            .module
            .bind_attested("tinychannels")
            .await
            .map_err(|failure| {
                ChannelRuntimeError::module(ChannelRuntimeOperation::Bind, failure.kind)
            })?;
        Ok(ChannelRuntimeDriver { bound, start })
    }

    /// List Discord guilds visible to the configured bot.
    ///
    /// `bot_token` and `proxy` are sent only after the module proves the
    /// registry-pinned release archive and extracted library on the same
    /// unique bus peer. Remote failure details are untrusted and may contain
    /// sensitive text; the returned error's `Debug` and `Display` are redacted.
    pub async fn discord_list_guilds(
        &self,
        config: ChannelsConfig,
        bot_token: String,
        proxy: HttpProxyPolicy,
    ) -> Result<Vec<DiscordGuild>, ModuleCallFailure> {
        let request = ChannelControllerRequest::DiscordListGuilds {
            config,
            bot_token,
            proxy,
        };
        self.discord(request, DiscordReply::Guilds).await
    }

    /// List Discord channels visible in a guild.
    pub async fn discord_list_channels(
        &self,
        config: ChannelsConfig,
        guild_id: String,
        bot_token: String,
        proxy: HttpProxyPolicy,
    ) -> Result<Vec<DiscordTextChannel>, ModuleCallFailure> {
        let request = ChannelControllerRequest::DiscordListChannels {
            config,
            guild_id,
            bot_token,
            proxy,
        };
        self.discord(request, DiscordReply::Channels).await
    }

    /// Check the configured Discord bot's permissions in a channel.
    pub async fn discord_check_permissions(
        &self,
        config: ChannelsConfig,
        guild_id: String,
        channel_id: String,
        bot_token: String,
        proxy: HttpProxyPolicy,
    ) -> Result<BotPermissionCheck, ModuleCallFailure> {
        let request = ChannelControllerRequest::DiscordCheckPermissions {
            config,
            guild_id,
            channel_id,
            bot_token,
            proxy,
        };
        self.discord(request, DiscordReply::Permissions).await
    }

    /// Check provider health using the timeout budget chosen by the host.
    ///
    /// The call is confidential because provider configuration and proxy
    /// policy can contain sensitive operational details.
    pub async fn check_health(
        &self,
        config: ChannelsConfig,
        proxy: HttpProxyPolicy,
        timeout_ms: u64,
    ) -> Result<Vec<tinychannels_bus::ChannelHealthReport>, ModuleCallFailure> {
        self.module
            .call_attested_detailed(
                "tinychannels",
                methods::CHECK_CHANNEL_HEALTH,
                (config, proxy, timeout_ms),
            )
            .await
    }

    async fn discord<T: DeserializeOwned>(
        &self,
        request: ChannelControllerRequest,
        expected: DiscordReply,
    ) -> Result<T, ModuleCallFailure> {
        let response: ChannelControllerResult = self
            .module
            .call_attested_detailed("tinychannels", methods::CHANNEL_CONTROLLER, (request,))
            .await?;
        Self::decode_discord(response, expected).ok_or_else(|| self.malformed_reply())
    }

    fn malformed_reply(&self) -> ModuleCallFailure {
        self.module.report_malformed_reply("tinychannels");
        ModuleCallFailure {
            kind: ModuleCallError::ModuleFault,
            wire_name: "ai.tinyhumans.tinybus.Error.Failed".into(),
            message: "malformed TinyChannels module response".into(),
        }
    }

    fn decode_discord<T: DeserializeOwned>(
        response: ChannelControllerResult,
        expected: DiscordReply,
    ) -> Option<T> {
        let raw = match (expected, response) {
            (DiscordReply::Guilds, ChannelControllerResult::DiscordGuilds(result)) => result.raw,
            (DiscordReply::Channels, ChannelControllerResult::DiscordChannels(result)) => {
                result.raw
            }
            (DiscordReply::Permissions, ChannelControllerResult::DiscordPermissions(result)) => {
                result.raw
            }
            _ => None,
        };
        raw.and_then(|value| serde_json::from_value(value).ok())
    }
}

/// A closed, content-free classification for one runtime operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelRuntimeOperation {
    /// Bind to the registered, attested TinyChannels peer.
    Bind,
    /// Reserve one module-owned runtime.
    Reserve,
    /// Validate the module-generated reservation.
    ValidateReservation,
    /// Construct the reservation's private module object.
    Object,
    /// Construct the returned callback name and path.
    CallbackAddress,
    /// Start the reserved runtime.
    Start,
    /// Stop the reserved runtime.
    Stop,
    /// Describe configured channel providers.
    DescribeConfiguredChannels,
    /// Start a supervised listener provider.
    StartChannelSupervised,
    /// Start an outbound-only provider.
    StartOutboundChannel,
    /// Prepare a channel control response.
    PrepareChannelControl,
    /// Execute a progressive reply operation.
    ExecuteProgressive,
    /// Prepare a typed outbound intent.
    PrepareOutboundIntent,
    /// Run an interaction on a configured provider.
    ChannelInteraction,
    /// Start a reserved relay and read its capability descriptor.
    StartRelay,
    /// Reserve an outbound relay.
    ReserveRelay,
    /// Stop a reserved relay.
    StopRelay,
    /// Read one provider's status.
    ChannelStatus,
    /// Send one provider intent.
    SendChannelIntent,
    /// Send one relay intent.
    SendRelayIntent,
    /// Shut down module-owned channel services on this object.
    Shutdown,
}

/// A closed error kind. It carries SDK classifications, never peer-provided text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelRuntimeErrorKind {
    /// The SDK classified the operation failure.
    Module(ModuleCallError),
    /// The reservation decoded but failed the bus contract's validation.
    InvalidReservation,
    /// The module-returned callback address is not a valid TinyBus address.
    InvalidCallbackAddress,
}

/// A runtime operation error that retains its typed cause without remote text.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ChannelRuntimeError {
    operation: ChannelRuntimeOperation,
    kind: ChannelRuntimeErrorKind,
}

impl ChannelRuntimeError {
    fn module(operation: ChannelRuntimeOperation, kind: ModuleCallError) -> Self {
        Self {
            operation,
            kind: ChannelRuntimeErrorKind::Module(kind),
        }
    }

    fn contract(operation: ChannelRuntimeOperation, kind: ChannelRuntimeErrorKind) -> Self {
        Self { operation, kind }
    }

    /// The operation that failed.
    #[must_use]
    pub fn operation(self) -> ChannelRuntimeOperation {
        self.operation
    }

    /// The closed SDK or contract classification.
    #[must_use]
    pub fn kind(self) -> ChannelRuntimeErrorKind {
        self.kind
    }
}

impl fmt::Debug for ChannelRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChannelRuntimeError")
            .field("operation", &self.operation)
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for ChannelRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TinyChannels runtime {:?} failed ({:?})",
            self.operation, self.kind
        )
    }
}

impl std::error::Error for ChannelRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ChannelRuntimeErrorKind::Module(error) => Some(error),
            ChannelRuntimeErrorKind::InvalidReservation
            | ChannelRuntimeErrorKind::InvalidCallbackAddress => None,
        }
    }
}

#[derive(Clone, Copy)]
enum RuntimeStart {
    Serial,
    Dispatch(ChannelRuntimeDispatchPolicy),
}

/// Runtime protocol driver bound to one attested TinyChannels peer.
///
/// Its bound client stays private so callers cannot bypass the typed protocol
/// projection or resolve the module alias again.
#[derive(Clone)]
pub struct ChannelRuntimeDriver {
    bound: BoundModuleClient,
    start: RuntimeStart,
}

#[async_trait]
impl RuntimeLeaseDriver for ChannelRuntimeDriver {
    type Reservation = ChannelRuntimeReservation;
    type Object = ChannelRuntimeClient;
    type Error = ChannelRuntimeError;

    async fn reserve(&self) -> Result<Self::Reservation, Self::Error> {
        self.bound
            .call_confidential(methods::RESERVE_RUNTIME, ())
            .await
            .map_err(|error| ChannelRuntimeError::module(ChannelRuntimeOperation::Reserve, error))
    }

    fn validate(&self, reservation: &Self::Reservation) -> Result<(), Self::Error> {
        reservation.validate().map_err(|_| {
            ChannelRuntimeError::contract(
                ChannelRuntimeOperation::ValidateReservation,
                ChannelRuntimeErrorKind::InvalidReservation,
            )
        })
    }

    fn callback_address(
        &self,
        reservation: &Self::Reservation,
        _object: &Self::Object,
    ) -> Result<(BusName, ObjectPath), Self::Error> {
        self.validate(reservation)?;
        let name = BusName::new(tinychannels_bus::HOST_BUS_NAME).map_err(|_| {
            ChannelRuntimeError::contract(
                ChannelRuntimeOperation::CallbackAddress,
                ChannelRuntimeErrorKind::InvalidCallbackAddress,
            )
        })?;
        let path = ObjectPath::new(&reservation.host_object_path).map_err(|_| {
            ChannelRuntimeError::contract(
                ChannelRuntimeOperation::CallbackAddress,
                ChannelRuntimeErrorKind::InvalidCallbackAddress,
            )
        })?;
        Ok((name, path))
    }

    async fn object(&self, reservation: &Self::Reservation) -> Result<Self::Object, Self::Error> {
        let object = self
            .bound
            .object_client(&reservation.object_path)
            .map_err(|error| ChannelRuntimeError::module(ChannelRuntimeOperation::Object, error))?;
        Ok(ChannelRuntimeClient { object })
    }

    async fn start(&self, reservation: &Self::Reservation) -> StartOutcome<Self::Error> {
        let result = match self.start {
            RuntimeStart::Serial => {
                self.bound
                    .call_confidential::<()>(methods::START_RUNTIME, (reservation.handle.clone(),))
                    .await
            }
            RuntimeStart::Dispatch(policy) => {
                self.bound
                    .call_confidential::<()>(
                        methods::START_RUNTIME_WITH_DISPATCH,
                        (reservation.handle.clone(), policy),
                    )
                    .await
            }
        };
        match result {
            Ok(()) => StartOutcome::Started,
            Err(error) => StartOutcome::Unacknowledged(ChannelRuntimeError::module(
                ChannelRuntimeOperation::Start,
                error,
            )),
        }
    }

    async fn stop(&self, reservation: &Self::Reservation) -> StopOutcome<Self::Error> {
        match self
            .bound
            .call_confidential::<()>(methods::STOP_RUNTIME, (reservation.handle.clone(),))
            .await
        {
            Ok(()) => StopOutcome::Acknowledged,
            Err(error) => StopOutcome::Unacknowledged(ChannelRuntimeError::module(
                ChannelRuntimeOperation::Stop,
                error,
            )),
        }
    }

    fn failed(&self, _reservation: &Self::Reservation) -> bool {
        // The current protocol has no authoritative terminal-fault observation.
        false
    }
}

/// Typed methods on one reservation-owned TinyChannels object.
///
/// The underlying SDK object client remains private. Each method preserves the
/// bus contract's argument order, response type, and delivery classification.
#[derive(Clone)]
pub struct ChannelRuntimeClient {
    object: ModuleObjectClient,
}

impl ChannelRuntimeClient {
    async fn confidential<R: DeserializeOwned>(
        &self,
        operation: ChannelRuntimeOperation,
        member: &str,
        args: impl serde::Serialize,
    ) -> Result<R, ChannelRuntimeError> {
        self.object
            .call_confidential(member, args)
            .await
            .map_err(|error| ChannelRuntimeError::module(operation, error))
    }

    async fn ordinary<R: DeserializeOwned>(
        &self,
        operation: ChannelRuntimeOperation,
        member: &str,
        args: impl serde::Serialize,
    ) -> Result<R, ChannelRuntimeError> {
        self.object
            .call(member, args)
            .await
            .map_err(|error| ChannelRuntimeError::module(operation, error))
    }

    /// Describe available providers without starting them.
    pub async fn describe_configured_channels(
        &self,
        config: ChannelsConfig,
        proxy: HttpProxyPolicy,
    ) -> Result<Vec<tinychannels_bus::module_runtime::ConfiguredChannelInfo>, ChannelRuntimeError>
    {
        self.confidential(
            ChannelRuntimeOperation::DescribeConfiguredChannels,
            methods::DESCRIBE_CONFIGURED_CHANNELS,
            (config, proxy),
        )
        .await
    }

    /// Start one supervised listener provider with host-selected retry timing.
    pub async fn start_channel_supervised(
        &self,
        name: String,
        config: ChannelsConfig,
        proxy: HttpProxyPolicy,
        policy: ChannelSupervisionPolicy,
    ) -> Result<(), ChannelRuntimeError> {
        self.confidential(
            ChannelRuntimeOperation::StartChannelSupervised,
            methods::START_CHANNEL_SUPERVISED,
            (name, config, proxy, policy),
        )
        .await
    }

    /// Start one external-webhook provider for outbound operations.
    pub async fn start_outbound_channel(
        &self,
        name: String,
        config: ChannelsConfig,
        proxy: HttpProxyPolicy,
    ) -> Result<(), ChannelRuntimeError> {
        self.confidential(
            ChannelRuntimeOperation::StartOutboundChannel,
            methods::START_OUTBOUND_CHANNEL,
            (name, config, proxy),
        )
        .await
    }

    /// Prepare a host-visible channel control response.
    pub async fn prepare_channel_control(
        &self,
        request: ChannelControlRequest,
    ) -> Result<ChannelControlResult, ChannelRuntimeError> {
        self.confidential(
            ChannelRuntimeOperation::PrepareChannelControl,
            methods::PREPARE_CHANNEL_CONTROL,
            (request,),
        )
        .await
    }

    /// Execute one progressive reply operation.
    pub async fn execute_progressive(
        &self,
        command: ProgressiveCommand,
    ) -> Result<ProgressiveResult, ChannelRuntimeError> {
        self.confidential(
            ChannelRuntimeOperation::ExecuteProgressive,
            methods::EXECUTE_PROGRESSIVE,
            (command,),
        )
        .await
    }

    /// Prepare one outbound intent inside TinyChannels.
    pub async fn prepare_outbound_intent(
        &self,
        channel: String,
        preparation: tinychannels_bus::module_runtime::ChannelMessagePreparation,
    ) -> Result<tinychannels_bus::module_runtime::PreparedChannelIntent, ChannelRuntimeError> {
        self.confidential(
            ChannelRuntimeOperation::PrepareOutboundIntent,
            methods::PREPARE_OUTBOUND_INTENT,
            (channel, preparation),
        )
        .await
    }

    /// Run a typed interaction on one configured provider.
    pub async fn channel_interaction(
        &self,
        request: ChannelInteractionRequest,
    ) -> Result<ChannelInteractionResult, ChannelRuntimeError> {
        self.confidential(
            ChannelRuntimeOperation::ChannelInteraction,
            methods::CHANNEL_INTERACTION,
            (request,),
        )
        .await
    }

    /// Start a relay and return the module's typed capability descriptor.
    pub async fn start_relay(
        &self,
        reservation: RelayReservation,
        config: tinychannels_bus::config::RelayRuntimeConfig,
    ) -> Result<tinychannels_bus::relay::CapabilityDescriptor, ChannelRuntimeError> {
        self.confidential(
            ChannelRuntimeOperation::StartRelay,
            methods::START_RELAY,
            (reservation, config),
        )
        .await
    }

    /// Reserve an outbound relay handle.
    pub async fn reserve_relay(&self) -> Result<RelayReservation, ChannelRuntimeError> {
        self.ordinary(
            ChannelRuntimeOperation::ReserveRelay,
            methods::RESERVE_RELAY,
            (),
        )
        .await
    }

    /// Stop one reserved relay.
    pub async fn stop_relay(
        &self,
        reservation: RelayReservation,
    ) -> Result<(), ChannelRuntimeError> {
        self.ordinary(
            ChannelRuntimeOperation::StopRelay,
            methods::STOP_RELAY,
            (reservation,),
        )
        .await
    }

    /// Read one provider's status.
    pub async fn channel_status(&self, name: String) -> Result<String, ChannelRuntimeError> {
        self.ordinary(
            ChannelRuntimeOperation::ChannelStatus,
            methods::CHANNEL_STATUS,
            (name,),
        )
        .await
    }

    /// Send one typed intent through a configured provider.
    pub async fn send_channel_intent(
        &self,
        name: String,
        intent: ChannelOutboundIntent,
    ) -> Result<ChannelSendMessageResult, ChannelRuntimeError> {
        self.ordinary(
            ChannelRuntimeOperation::SendChannelIntent,
            methods::SEND_CHANNEL_INTENT,
            (name, intent),
        )
        .await
    }

    /// Send one typed intent through a reserved relay.
    pub async fn send_relay_intent(
        &self,
        reservation: RelayReservation,
        intent: ChannelOutboundIntent,
    ) -> Result<ChannelSendMessageResult, ChannelRuntimeError> {
        self.ordinary(
            ChannelRuntimeOperation::SendRelayIntent,
            methods::SEND_RELAY_INTENT,
            (reservation, intent),
        )
        .await
    }

    /// Shut down provider and relay services owned by this runtime object.
    pub async fn shutdown(&self) -> Result<(), ChannelRuntimeError> {
        self.ordinary(ChannelRuntimeOperation::Shutdown, methods::SHUTDOWN, ())
            .await
    }
}

#[derive(Clone, Copy)]
enum DiscordReply {
    Guilds,
    Channels,
    Permissions,
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
