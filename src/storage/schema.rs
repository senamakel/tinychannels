//! Collection declarations owned by TinyChannels.
//!
//! The host consumes the same TSD2 declarations as the storage implementation,
//! so collection names and shapes cannot drift between the bus contract and
//! the owner-local migration code.

pub use tinychannels_bus::storage::schema::*;
