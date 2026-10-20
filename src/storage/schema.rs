//! What TinyChannels stores, declared for the host's schema registry.
//!
//! [`storage_schema`] is the one declaration of the `channels` owner: every
//! collection the delivery queue, the remote-control session store and the
//! WhatsApp Web session backend write, plus the steps that bring the v1
//! collections (`channels_delivery_*`, `channel_remote_*`, `whatsapp_*`)
//! forward onto the v2 names. A host builds
//! `Registry::compose([tinychannels::storage::storage_schema(), ..])`, prepares
//! the backend and migrates; the bus contract separately exposes the static
//! `StorageNeeds` inventory.
//!
//! The WhatsApp collections are declared whether or not the `whatsapp-web`
//! feature is compiled in: the grant and the migration must not depend on how
//! this crate was built.

use tinystoragedrivers_core::schema::{Owner, Schema, ScopeClass};

use super::migrate;

pub use tinychannels_bus::storage_names::{
    COLLECTIONS, DELIVERY_FAILED, DELIVERY_PENDING, DELIVERY_RESULTS, REMOTE_BINDINGS, REMOTE_BUSY,
    REMOTE_META, WHATSAPP_COLLECTIONS, WHATSAPP_DEVICE, WHATSAPP_IDENTITY, WHATSAPP_LID_MAPPING,
    WHATSAPP_PREKEY, WHATSAPP_SENDER_KEY, WHATSAPP_SESSION, WHATSAPP_SIGNED_PREKEY,
    WHATSAPP_SYNC_KEY,
};

/// The owner every collection below is prefixed with.
pub const OWNER: &str = "channels";

/// Body version of every `channels.*` collection. Version 1 is the pre-registry
/// layout stored under the legacy names; the migration steps write 2.
pub const SCHEMA_VERSION: u32 = 2;

/// The `channels` owner's declaration: its collections and the v1 to v2 steps.
///
/// Every item is [`ScopeClass::Module`]: the records belong to the channels
/// module inside a tenant, never to the tenant as a whole.
#[must_use]
pub fn storage_schema() -> Schema {
    let owner = Owner::new(OWNER).unwrap_or_else(|_| unreachable!("`{OWNER}` is a valid owner"));
    let schema = COLLECTIONS.iter().fold(Schema::new(owner), |schema, name| {
        schema.collection(name, SCHEMA_VERSION, ScopeClass::Module)
    });
    migrate::steps()
        .into_iter()
        .fold(schema, |schema, step| schema.step(step))
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
