use super::*;

#[test]
fn provider_event_retains_target_message_and_event_identity() {
    let event = ChannelEvent {
        event_id: "event-1".into(),
        channel: ChannelRef {
            id: "telegram".into(),
            account_id: None,
        },
        conversation: Some(ConversationRef {
            id: "chat-1".into(),
            ..Default::default()
        }),
        event: ChannelEventKind::ReactionAdded {
            message_id: "message-2".into(),
            reaction: ChannelReaction {
                value: "👍".into(),
                custom: false,
                custom_emoji_id: None,
            },
            actor_id: Some("user-3".into()),
        },
        timestamp_unix_ms: Some(12),
    };
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["event"]["kind"], "reaction_added");
    assert_eq!(
        serde_json::from_value::<ChannelEvent>(value).unwrap(),
        event
    );
}

#[test]
fn read_receipt_identity_and_custom_emoji_survive_round_trip() {
    let event = ChannelEvent {
        event_id: "event-read".into(),
        channel: ChannelRef {
            id: "telegram".into(),
            account_id: None,
        },
        conversation: None,
        event: ChannelEventKind::ReadReceipt {
            message_id: "message-1".into(),
            actor_id: Some("reader-1".into()),
            read_at_unix_ms: Some(42),
        },
        timestamp_unix_ms: Some(43),
    };
    let encoded = serde_json::to_value(&event).unwrap();
    assert_eq!(encoded["event"]["actor_id"], "reader-1");
    assert_eq!(
        serde_json::from_value::<ChannelEvent>(encoded).unwrap(),
        event
    );

    let reaction = ChannelReaction {
        value: "fire".into(),
        custom: true,
        custom_emoji_id: Some("provider-emoji-9".into()),
    };
    let encoded = serde_json::to_value(&reaction).unwrap();
    assert_eq!(encoded["customEmojiId"], "provider-emoji-9");
    assert_eq!(
        serde_json::from_value::<ChannelReaction>(encoded).unwrap(),
        reaction
    );

    let legacy = serde_json::json!({"value":"👍","custom":false});
    assert_eq!(
        serde_json::from_value::<ChannelReaction>(legacy).unwrap(),
        ChannelReaction {
            value: "👍".into(),
            custom: false,
            custom_emoji_id: None,
        }
    );
}
