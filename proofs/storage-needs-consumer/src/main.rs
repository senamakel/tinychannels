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
            "channels.delivery_pending",
            "channels.delivery_failed",
            "channels.delivery_results",
            "channels.remote_bindings",
            "channels.remote_busy",
            "channels.remote_meta",
            "channels.whatsapp_device",
            "channels.whatsapp_identity",
            "channels.whatsapp_session",
            "channels.whatsapp_prekey",
            "channels.whatsapp_signed_prekey",
            "channels.whatsapp_sender_key",
            "channels.whatsapp_sync_key",
            "channels.whatsapp_lid_mapping",
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
