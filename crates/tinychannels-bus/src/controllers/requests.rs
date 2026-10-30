//! Typed requests accepted by the public channel controller surface.
use super::definitions::ChannelAuthMode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

macro_rules! request {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
        #[serde(default, rename_all = "camelCase")]
        pub struct $name { $(pub $field: $ty,)* }
    };
}

request!(ChannelDescribeRequest { channel: String });
request!(ChannelConnectRequest { channel: String, auth_mode: ChannelAuthMode, credentials: Option<Value> });
request!(ChannelDisconnectRequest {
    channel: String,
    auth_mode: ChannelAuthMode,
    clear_memory: bool
});
request!(ChannelStatusRequest { channel: Option<String> });
request!(ChannelTestRequest {
    channel: String,
    auth_mode: ChannelAuthMode,
    credentials: Value
});
request!(ChannelSendMessageRequest {
    channel: String,
    message: Value
});
request!(ChannelSendReactionRequest {
    channel: String,
    reaction: Value
});
request!(ChannelCreateThreadRequest {
    channel: String,
    title: String
});
request!(ChannelUpdateThreadRequest {
    channel: String,
    thread_id: String,
    action: String
});
request!(ChannelListThreadsRequest { channel: String, active: Option<bool> });
request!(DiscordListChannelsRequest { guild_id: String });
request!(DiscordCheckPermissionsRequest {
    guild_id: String,
    channel_id: String
});
