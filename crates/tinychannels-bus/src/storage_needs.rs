//! Storage resources requested by the TinyChannels module.
//!
//! This is declaration data only. It does not grant access or provide a
//! storage implementation; the host applies its own policy to this inventory.

use std::sync::LazyLock;

use tinystoragedrivers_bus::{CollectionSpec, StorageNeeds};

/// Delivery queue collection for pending entries.
pub const DELIVERY_PENDING_COLLECTION: &str = "channels_delivery_pending";
/// Delivery queue collection for permanently failed entries.
pub const DELIVERY_FAILED_COLLECTION: &str = "channels_delivery_failed";
/// Delivery queue collection for terminal idempotency results.
pub const DELIVERY_RESULTS_COLLECTION: &str = "channels_delivery_results";

/// Remote-control collection for chat-to-thread bindings.
pub const REMOTE_BINDINGS_COLLECTION: &str = "channel_remote_bindings";
/// Remote-control collection for in-flight chat turns.
pub const REMOTE_BUSY_COLLECTION: &str = "channel_remote_busy";
/// Remote-control collection for the legacy-import marker.
pub const REMOTE_META_COLLECTION: &str = "channel_remote_meta";

/// WhatsApp Web collection for linked-device state.
pub const WHATSAPP_DEVICE_COLLECTION: &str = "whatsapp_device";
/// WhatsApp Web collection for identity records.
pub const WHATSAPP_IDENTITY_COLLECTION: &str = "whatsapp_identity";
/// WhatsApp Web collection for Signal sessions.
pub const WHATSAPP_SESSION_COLLECTION: &str = "whatsapp_session";
/// WhatsApp Web collection for pre-keys.
pub const WHATSAPP_PREKEY_COLLECTION: &str = "whatsapp_prekey";
/// WhatsApp Web collection for signed pre-keys.
pub const WHATSAPP_SIGNED_PREKEY_COLLECTION: &str = "whatsapp_signed_prekey";
/// WhatsApp Web collection for sender keys.
pub const WHATSAPP_SENDER_KEY_COLLECTION: &str = "whatsapp_sender_key";
/// WhatsApp Web collection for app-state synchronization keys.
pub const WHATSAPP_SYNC_KEY_COLLECTION: &str = "whatsapp_sync_key";
/// WhatsApp Web collection for LID-to-phone mappings.
pub const WHATSAPP_LID_MAPPING_COLLECTION: &str = "whatsapp_lid_mapping";

/// All collections written by the durable WhatsApp Web backend.
pub const WHATSAPP_COLLECTIONS: [&str; 8] = [
    WHATSAPP_DEVICE_COLLECTION,
    WHATSAPP_IDENTITY_COLLECTION,
    WHATSAPP_SESSION_COLLECTION,
    WHATSAPP_PREKEY_COLLECTION,
    WHATSAPP_SIGNED_PREKEY_COLLECTION,
    WHATSAPP_SENDER_KEY_COLLECTION,
    WHATSAPP_SYNC_KEY_COLLECTION,
    WHATSAPP_LID_MAPPING_COLLECTION,
];

static COLLECTIONS: LazyLock<[CollectionSpec; 14]> = LazyLock::new(|| {
    [
        CollectionSpec::new(DELIVERY_PENDING_COLLECTION),
        CollectionSpec::new(DELIVERY_FAILED_COLLECTION),
        CollectionSpec::new(DELIVERY_RESULTS_COLLECTION),
        CollectionSpec::new(REMOTE_BINDINGS_COLLECTION),
        CollectionSpec::new(REMOTE_BUSY_COLLECTION),
        CollectionSpec::new(REMOTE_META_COLLECTION),
        CollectionSpec::new(WHATSAPP_DEVICE_COLLECTION),
        CollectionSpec::new(WHATSAPP_IDENTITY_COLLECTION),
        CollectionSpec::new(WHATSAPP_SESSION_COLLECTION),
        CollectionSpec::new(WHATSAPP_PREKEY_COLLECTION),
        CollectionSpec::new(WHATSAPP_SIGNED_PREKEY_COLLECTION),
        CollectionSpec::new(WHATSAPP_SENDER_KEY_COLLECTION),
        CollectionSpec::new(WHATSAPP_SYNC_KEY_COLLECTION),
        CollectionSpec::new(WHATSAPP_LID_MAPPING_COLLECTION),
    ]
});

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
