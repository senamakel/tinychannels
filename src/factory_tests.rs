//! Tests for the provider factory.
//!
//! These matter more than their size suggests: this function is the single
//! config-to-provider mapping shared by the OpenHuman core and the
//! `tinychannels-module` cdylib. Before it existed the mapping was inline in
//! the host, so "does a config stanza produce a provider" was only ever
//! answered by starting the real thing.

use crate::factory::{DefaultHttpClients, build_channels};
use crate::host::ChannelHost;
use crate::NoopHost;
use std::sync::Arc;
use tinychannels_bus::ChannelsConfig;
use tinychannels_bus::config::{DiscordConfig, TelegramConfig, WhatsAppConfig};

fn host() -> Arc<dyn ChannelHost> {
    NoopHost::arc()
}

fn names(config: &ChannelsConfig) -> Vec<String> {
    build_channels(config, &host(), &DefaultHttpClients)
        .iter()
        .map(|channel| channel.name().to_owned())
        .collect()
}

fn telegram() -> TelegramConfig {
    TelegramConfig {
        bot_token: "token".to_owned(),
        chat_id: None,
        allowed_users: Vec::new(),
        stream_mode: crate::config::StreamMode::default(),
        draft_update_interval_ms: 1_000,
        silent_streaming: false,
        mention_only: false,
    }
}

#[test]
fn a_default_config_builds_no_channels() {
    assert!(names(&ChannelsConfig::default()).is_empty());
}

#[test]
fn a_configured_provider_is_built() {
    let mut config = ChannelsConfig::default();
    config.telegram = Some(telegram());
    assert_eq!(names(&config), vec!["telegram".to_owned()]);
}

/// Order is part of the contract: a caller that indexes the result, or logs it,
/// should see the same sequence across runs and across the two hosts.
#[test]
fn providers_are_returned_in_declaration_order() {
    let mut config = ChannelsConfig::default();
    config.discord = Some(DiscordConfig {
        bot_token: "token".to_owned(),
        guild_id: String::new(),
        channel_id: String::new(),
        allowed_users: Vec::new(),
        listen_to_bots: false,
        mention_only: false,
    });
    config.telegram = Some(telegram());

    // Telegram is declared before Discord in the factory, so it comes first
    // regardless of the order the fields were set here.
    assert_eq!(
        names(&config),
        vec!["telegram".to_owned(), "discord".to_owned()]
    );
}

/// A WhatsApp stanza that is neither a Cloud nor a Web shape is skipped.
///
/// The important half is that it does **not** panic and does **not** take the
/// other providers down with it: one bad stanza must not cost a user every
/// other channel.
#[test]
fn an_unusable_whatsapp_stanza_is_skipped_without_disturbing_others() {
    let mut config = ChannelsConfig::default();
    config.telegram = Some(telegram());
    config.whatsapp = Some(WhatsAppConfig::default());

    assert_eq!(names(&config), vec!["telegram".to_owned()]);
}
