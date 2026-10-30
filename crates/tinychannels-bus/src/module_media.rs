//! Bounded, scoped media upload and read payloads for module callbacks.
use crate::{HostVoiceScope, MediaKind, MediaReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MEDIA_READ_CHUNK_BYTES: usize = 256 * 1024;
pub const MEDIA_UPLOAD_CHUNK_BYTES: usize = 256 * 1024;
pub const MEDIA_UPLOAD_MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MEDIA_UPLOAD_BEGIN_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct MediaUploadHandle(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct MediaReadHandle(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaUploadBegin {
    pub instance: String,
    pub scope: HostVoiceScope,
    pub kind: MediaKind,
    pub content_type: String,
    pub file_name: Option<String>,
    pub duration_ms: Option<u64>,
    pub codec: Option<String>,
    pub byte_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaUploadChunk { pub handle: MediaUploadHandle, pub sequence: u32, pub bytes: Vec<u8> }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct MediaUploadFinish(pub MediaUploadHandle);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaReadBegin { pub instance: String, pub scope: HostVoiceScope, pub media: MediaReference, pub max_bytes: u64 }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaReadInfo {
    pub handle: MediaReadHandle,
    pub media: MediaReference,
    pub content_type: String,
    pub byte_count: usize,
    pub chunk_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaReadChunk { pub handle: MediaReadHandle, pub sequence: u32 }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaReadChunkResult { pub sequence: u32, pub bytes: Vec<u8>, pub done: bool }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct MediaReadFinish(pub MediaReadHandle);
