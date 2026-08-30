//! TinyChannels bus identity and member names.
//!
//! Two objects, in opposite directions. The module serves [`BUS_NAME`] so a host
//! can drive providers; the *host* serves [`HOST_BUS_NAME`] so the module can
//! deliver what arrives from the network. A channel is bidirectional and a
//! served object cannot open a stream back to its caller, so inbound traffic
//! needs its own host-side object rather than a reply on the outbound call.

/// Well-known bus name exported by the TinyChannels module.
pub const BUS_NAME: &str = "ai.tinyhumans.tinychannels.Channels";

/// Object path served by the TinyChannels module.
pub const OBJECT_PATH: &str = "/ai/tinyhumans/tinychannels/Channels";

/// Well-known bus name the *host* serves for module-to-host callbacks.
pub const HOST_BUS_NAME: &str = "ai.tinyhumans.tinychannels.ChannelsHost";

/// Object path the host serves for module-to-host callbacks.
pub const HOST_OBJECT_PATH: &str = "/ai/tinyhumans/tinychannels/ChannelsHost";

/// One constant per method name on [`BUS_NAME`].
pub mod methods {
    /// `StartChannel` — connect and begin receiving on one configured provider.
    pub const START_CHANNEL: &str = "StartChannel";
    /// `StopChannel` — disconnect one running provider.
    pub const STOP_CHANNEL: &str = "StopChannel";
    /// `SendMessage` — deliver one outbound intent through a running provider.
    pub const SEND_MESSAGE: &str = "SendMessage";
    /// `ListChannels` — report every provider this build can serve.
    pub const LIST_CHANNELS: &str = "ListChannels";
    /// `ChannelStatus` — report connection state for one provider.
    pub const CHANNEL_STATUS: &str = "ChannelStatus";
}

/// One constant per method name on [`HOST_BUS_NAME`].
pub mod host_methods {
    /// `DeliverInbound` — hand the host one authenticated inbound envelope.
    pub const DELIVER_INBOUND: &str = "DeliverInbound";
    /// `ReportStatus` — report a provider connection state transition.
    pub const REPORT_STATUS: &str = "ReportStatus";
}

/// All module method names in the declaration order used by the interface.
pub const METHODS: [&str; 5] = [
    methods::START_CHANNEL,
    methods::STOP_CHANNEL,
    methods::SEND_MESSAGE,
    methods::LIST_CHANNELS,
    methods::CHANNEL_STATUS,
];

/// All host-callback method names in declaration order.
pub const HOST_METHODS: [&str; 2] = [host_methods::DELIVER_INBOUND, host_methods::REPORT_STATUS];

#[cfg(test)]
mod tests {
    use super::{
        BUS_NAME, HOST_BUS_NAME, HOST_METHODS, HOST_OBJECT_PATH, METHODS, OBJECT_PATH,
        host_methods, methods,
    };

    /// `METHODS` is what a host iterates to build its interface, so a member
    /// added to `methods` but forgotten here is a method the module serves and
    /// no host ever binds — a `MemberNotFound` in the field, not a build error.
    #[test]
    fn every_declared_member_appears_in_its_method_table() {
        for m in [
            methods::START_CHANNEL,
            methods::STOP_CHANNEL,
            methods::SEND_MESSAGE,
            methods::LIST_CHANNELS,
            methods::CHANNEL_STATUS,
        ] {
            assert!(METHODS.contains(&m), "{m} missing from METHODS");
        }
        for m in [host_methods::DELIVER_INBOUND, host_methods::REPORT_STATUS] {
            assert!(HOST_METHODS.contains(&m), "{m} missing from HOST_METHODS");
        }
    }

    /// A duplicate silently shadows one member with another's handler.
    #[test]
    fn member_names_are_unique_within_each_object() {
        let mut sorted = METHODS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), METHODS.len(), "duplicate member in METHODS");
    }

    /// The two objects must not collide: the module serves one, the host the
    /// other, on the same broker.
    #[test]
    fn the_module_and_host_objects_are_distinct() {
        assert_ne!(BUS_NAME, HOST_BUS_NAME);
        assert_ne!(OBJECT_PATH, HOST_OBJECT_PATH);
    }

    /// Bus names are dotted and object paths are their slashed form. A host
    /// that derives one from the other must keep agreeing with these literals.
    #[test]
    fn object_paths_are_the_slashed_form_of_their_bus_names() {
        assert_eq!(OBJECT_PATH, format!("/{}", BUS_NAME.replace('.', "/")));
        assert_eq!(
            HOST_OBJECT_PATH,
            format!("/{}", HOST_BUS_NAME.replace('.', "/"))
        );
    }
}
