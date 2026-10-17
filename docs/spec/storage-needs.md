# Storage needs

The `tinychannels-bus` contract declares the resources that OpenHuman must
grant to the TinyChannels module. The declaration is a pure contract value;
the host remains responsible for the storage implementation and access policy.

The current implementation persists 14 collections:

| Area | Collections |
| --- | --- |
| Delivery queue | `channels_delivery_pending`, `channels_delivery_failed`, `channels_delivery_results` |
| Remote session state | `channel_remote_bindings`, `channel_remote_busy`, `channel_remote_meta` |
| WhatsApp Web session | `whatsapp_device`, `whatsapp_identity`, `whatsapp_session`, `whatsapp_prekey`, `whatsapp_signed_prekey`, `whatsapp_sender_key`, `whatsapp_sync_key`, `whatsapp_lid_mapping` |

These names are declared by the implementation modules that use them. The bus
contract aggregates those declarations; host grants should consume that value
instead of copying the inventory. The audited implementation currently creates
no indexes, streams, blobs, or secret resources for these stores.
