# Storage needs

The `tinychannels-bus` crate exposes `STORAGE_NEEDS`, a data-only declaration
of the collections the TinyChannels module needs its host to provision. The
declaration does not grant access or implement storage. The host owns the
backend, access policy, and provisioning lifecycle.

The canonical collection names are defined in
[`tinychannels_bus::storage_names`](../../crates/tinychannels-bus/src/storage_names.rs).
Both the bus declaration and the implementation schema use this inventory.
The current inventory contains 14 `channels.*` collections:

| Area | Current collection names |
| --- | --- |
| Delivery queue | `channels.delivery_pending`, `channels.delivery_failed`, `channels.delivery_results` |
| Remote session state | `channels.remote_bindings`, `channels.remote_busy`, `channels.remote_meta` |
| WhatsApp Web session | `channels.whatsapp_device`, `channels.whatsapp_identity`, `channels.whatsapp_session`, `channels.whatsapp_prekey`, `channels.whatsapp_signed_prekey`, `channels.whatsapp_sender_key`, `channels.whatsapp_sync_key`, `channels.whatsapp_lid_mapping` |

The bus requests no indexes, TTL fields, search declarations, blobs, streams,
or secrets. The implementation's `storage_schema()` declares these same
collections under the `channels` owner as module-scoped schema version 2 items
([`src/storage/schema.rs`](../../src/storage/schema.rs)). The eight WhatsApp
collections remain in the contract and schema even when the `whatsapp-web`
feature is disabled, so provisioning and migration do not vary by build
feature.

## Legacy migration sources

The following unscoped names are the v1 source collections read by the
implementation's migration steps. They are not the current collection names
requested by `STORAGE_NEEDS` or declared in the v2 schema. Each step migrates
records from its v1 source to the corresponding v2 destination above
([`src/storage/migrate.rs`](../../src/storage/migrate.rs)).

| Area | Legacy v1 source names |
| --- | --- |
| Delivery queue | `channels_delivery_pending`, `channels_delivery_failed`, `channels_delivery_results` |
| Remote session state | `channel_remote_bindings`, `channel_remote_busy`, `channel_remote_meta` |
| WhatsApp Web session | `whatsapp_device`, `whatsapp_identity`, `whatsapp_session`, `whatsapp_prekey`, `whatsapp_signed_prekey`, `whatsapp_sender_key`, `whatsapp_sync_key`, `whatsapp_lid_mapping` |

The migration steps move each source from schema version 1 to version 2. They
preserve valid records in the new collections and apply the v2 record shape;
legacy collection names remain only as migration inputs.

[`proofs/storage-needs-consumer`](../../proofs/storage-needs-consumer) builds
outside the TinyChannels workspace and assigns the exported declaration to the
canonical `StorageNeeds` type. CI runs that consumer on Rust 1.94 and checks
its locked normal/build dependency graph to keep storage clients, drivers,
TinyBus, and non-sync Tokio features out of this contract boundary.
