//! Module-owned web output projection request and result DTOs.
use crate::{module_progressive::ChannelOutputEvent, web::WebEventContext};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct WebTurnHandle {
    pub handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BeginWebTurn {
    pub scope_key: String,
    pub context: WebEventContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EndWebTurn {
    pub handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWebOutput {
    pub handle: String,
    pub event: ChannelOutputEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebOutputEvents {
    pub events: Vec<crate::web::WebChannelEvent>,
}
