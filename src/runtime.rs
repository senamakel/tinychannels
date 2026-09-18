//! Runtime mechanics re-exported from the lightweight runtime crate.

pub use tinychannels_runtime::{
    ListenerObserver, MAX_JITTER_MS, NoopListenerObserver, compute_max_in_flight_messages,
    jitter_millis, select_acknowledgment_reaction, spawn_supervised_listener,
};
