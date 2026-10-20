//! Storage resources requested by the TinyChannels module.
//!
//! This is declaration data only. It does not grant access or provide a
//! storage implementation; the host applies its own policy to this inventory.

use std::sync::LazyLock;

use tinystoragedrivers_bus::{CollectionSpec, StorageNeeds};

pub use crate::storage_names::{
    DELIVERY_FAILED as DELIVERY_FAILED_COLLECTION, DELIVERY_PENDING as DELIVERY_PENDING_COLLECTION,
    DELIVERY_RESULTS as DELIVERY_RESULTS_COLLECTION, REMOTE_BINDINGS as REMOTE_BINDINGS_COLLECTION,
    REMOTE_BUSY as REMOTE_BUSY_COLLECTION, REMOTE_META as REMOTE_META_COLLECTION,
    WHATSAPP_COLLECTIONS, WHATSAPP_DEVICE as WHATSAPP_DEVICE_COLLECTION,
    WHATSAPP_IDENTITY as WHATSAPP_IDENTITY_COLLECTION,
    WHATSAPP_LID_MAPPING as WHATSAPP_LID_MAPPING_COLLECTION,
    WHATSAPP_PREKEY as WHATSAPP_PREKEY_COLLECTION,
    WHATSAPP_SENDER_KEY as WHATSAPP_SENDER_KEY_COLLECTION,
    WHATSAPP_SESSION as WHATSAPP_SESSION_COLLECTION,
    WHATSAPP_SIGNED_PREKEY as WHATSAPP_SIGNED_PREKEY_COLLECTION,
    WHATSAPP_SYNC_KEY as WHATSAPP_SYNC_KEY_COLLECTION,
};

static COLLECTIONS: LazyLock<[CollectionSpec; 14]> = LazyLock::new(|| {
    std::array::from_fn(|index| collection(crate::storage_names::COLLECTIONS[index]))
});

fn collection(name: &str) -> CollectionSpec {
    CollectionSpec {
        name: name.into(),
        indexes: Vec::new(),
        ttl_field: None,
        search: None,
    }
}

/// Resources TinyChannels requires from the host storage port.
///
/// The value lists the 14 collections used by delivery, remote-session, and
/// WhatsApp persistence. It requests no indexes, blobs, streams, or secrets.
pub static STORAGE_NEEDS: LazyLock<StorageNeeds<'static>> = LazyLock::new(|| StorageNeeds {
    collections: &*COLLECTIONS,
    blob_prefixes: &[],
    stream_prefixes: &[],
    secret_prefixes: &[],
});
