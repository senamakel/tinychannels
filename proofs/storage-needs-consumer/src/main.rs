use tinystoragedrivers_types::CollectionSpec;
use tinychannels_bus::storage::collections;

fn main() {
    let needs = collections();
    let names: Vec<_> = needs.iter().map(|collection| collection.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "channels.delivery_pending",
            "channels.delivery_failed",
            "channels.delivery_results",
            "channels.remote_bindings",
            "channels.remote_busy",
            "channels.remote_meta",
            "channels.media_metadata",
            "channels.migrations",
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
    assert_eq!(needs.len(), 16);
    let _: &CollectionSpec = &needs[0];
    assert!(needs.iter().all(|collection| {
        collection.indexes.is_empty()
            && collection.ttl_field.is_none()
            && collection.search.is_none()
    }));
}
