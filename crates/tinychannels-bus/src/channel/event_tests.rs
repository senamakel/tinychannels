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
