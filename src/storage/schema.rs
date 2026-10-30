//! Storage driver declarations adapted from TinyChannels' pure bus DTOs.
//!
//! The bus contract describes collection shapes with serde-only data types.
//! This implementation layer converts those declarations to TinyStorageDrivers
//! types where storage setup is performed.

use tinystoragedrivers_types::{
    CollectionSpec as DriverCollectionSpec, IndexSpec as DriverIndexSpec,
};

pub use tinychannels_bus::storage::schema::{
    COLLECTIONS, DELIVERY_FAILED, DELIVERY_PENDING, DELIVERY_RESULTS, MEDIA_BLOB_PREFIX,
    MEDIA_METADATA, MEDIA_METADATA_VERSION, MIGRATIONS, REMOTE_BINDINGS, REMOTE_BUSY, REMOTE_META,
    SCHEMA_VERSION, WHATSAPP_COLLECTIONS, WHATSAPP_DEVICE, WHATSAPP_IDENTITY, WHATSAPP_LID_MAPPING,
    WHATSAPP_PREKEY, WHATSAPP_SENDER_KEY, WHATSAPP_SESSION, WHATSAPP_SIGNED_PREKEY,
    WHATSAPP_SYNC_KEY,
};
pub use tinychannels_bus::storage::schema::{
    CollectionSpec as CollectionSchema, IndexSpec as IndexSchema, SearchSpec as SearchSchema,
};

/// Return the storage-driver declaration for media metadata.
#[must_use]
pub fn media_metadata_spec() -> DriverCollectionSpec {
    to_driver_spec(tinychannels_bus::storage::schema::media_metadata_spec())
}

/// Adapt the owner inventory to the storage driver's setup API.
#[must_use]
pub fn collections() -> Vec<DriverCollectionSpec> {
    tinychannels_bus::storage::schema::collections()
        .into_iter()
        .map(to_driver_spec)
        .collect()
}

fn to_driver_spec(spec: CollectionSchema) -> DriverCollectionSpec {
    let mut driver = DriverCollectionSpec::new(spec.name);
    for index in spec.indexes {
        let index = DriverIndexSpec::new(index.name, index.fields);
        driver = driver.index(if index.unique { index.unique() } else { index });
    }
    if let Some(ttl_field) = spec.ttl_field {
        driver = driver.ttl(ttl_field);
    }
    if let Some(search) = spec.search {
        driver = driver.searchable(search.fields);
    }
    driver
}
