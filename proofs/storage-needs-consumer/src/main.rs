use tinystoragedrivers_bus::StorageNeeds;
use tinychannels_bus::STORAGE_NEEDS;

fn main() {
    let needs: &StorageNeeds<'static> = &STORAGE_NEEDS;
    let names: Vec<_> = needs
        .collections
        .iter()
        .map(|collection| collection.name.as_str())
        .collect();

    assert_eq!(
        names,
        [
            "channels_delivery_pending",
            "channels_delivery_failed",
            "channels_delivery_results",
            "channel_remote_bindings",
            "channel_remote_busy",
            "channel_remote_meta",
            "whatsapp_device",
            "whatsapp_identity",
            "whatsapp_session",
            "whatsapp_prekey",
            "whatsapp_signed_prekey",
            "whatsapp_sender_key",
            "whatsapp_sync_key",
            "whatsapp_lid_mapping",
        ]
    );
    assert_eq!(needs.collections.len(), 14);
    assert!(needs.collections.iter().all(|collection| {
        collection.indexes.is_empty()
            && collection.ttl_field.is_none()
            && collection.search.is_none()
    }));
    assert!(needs.blob_prefixes.is_empty());
    assert!(needs.stream_prefixes.is_empty());
    assert!(needs.secret_prefixes.is_empty());
}
