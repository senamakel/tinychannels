//! Portable routing context for an approval reply sent over a channel.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ReplyContext {
    pub reply_target: String,
    pub thread_ts: Option<String>,
}
