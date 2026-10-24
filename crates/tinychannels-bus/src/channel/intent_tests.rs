use super::*;
use crate::channel::MediaKind;

#[test]
fn typed_parts_round_trip_with_order_and_addressing() {
    let message = ChannelOutboundMessage {
        channel_id: "signal:account".into(),
        conversation_id: "conversation".into(),
        channel_ref: Some(ChannelRef {
            id: "signal:account".into(),
            account_id: Some("account".into()),
        }),
        conversation_ref: Some(ConversationRef {
            id: "conversation".into(),
            thread_id: Some("thread".into()),
            topic_id: Some("topic".into()),
            ..Default::default()
        }),
        reply_to_id: Some("message".into()),
        thread_id: Some("thread".into()),
        turn_id: Some("turn".into()),
        idempotency_key: "send-1".into(),
        parts: vec![
            OutboundContentPart::Content {
                part: ContentPart::Text {
                    text: "caption".into(),
                    markdown_dialect: Some("commonmark".into()),
                },
            },
            OutboundContentPart::Content {
                part: ContentPart::Image {
                    media: MediaReference {
                        url: Some("https://example.test/image.png".into()),
                        kind: MediaKind::Image,
                        ..Default::default()
                    },
                    caption: Some("image caption".into()),
                    alt_text: Some("a blue square".into()),
                },
            },
        ],
    };

    let wire = serde_json::to_value(&message).unwrap();
    let decoded: ChannelOutboundMessage = serde_json::from_value(wire).unwrap();
    assert_eq!(decoded, message);
    assert_eq!(decoded.parts.len(), 2);
    assert!(decoded.conversation_ref.unwrap().topic_id.is_some());
}

#[test]
fn typed_adapter_preserves_rich_legacy_payloads_and_rejects_text_projection() {
    let intent = ChannelOutboundIntent {
        channel_id: "telegram".into(),
        conversation_id: "room".into(),
        idempotency_key: "send-2".into(),
        payload: OutboundPayload::Parts {
            parts: vec![OutboundContentPart::Content {
                part: ContentPart::Location {
                    latitude: 1.5,
                    longitude: 2.5,
                    label: Some("office".into()),
                    address: None,
                    accuracy_meters: None,
                    live_until_unix_ms: None,
                },
            }],
        },
        ..Default::default()
    };
    let typed = ChannelOutboundMessage::from_intent(&intent);
    assert_eq!(
        typed.parts,
        match intent.payload {
            OutboundPayload::Parts { parts } => parts,
            _ => unreachable!(),
        }
    );
    let error = typed.to_legacy_message().unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not support typed channel content")
    );
}

#[test]
fn legacy_payloads_adapt_to_typed_content_without_dropping_text() {
    let intent = ChannelOutboundIntent {
        payload: OutboundPayload::Media {
            text: Some("caption".into()),
            media_urls: vec!["https://example.test/image.png".into()],
        },
        ..Default::default()
    };
    let typed = ChannelOutboundMessage::from_intent(&intent);
    assert!(matches!(
        &typed.parts[0],
        OutboundContentPart::Content { part: ContentPart::Text { text, .. } } if text == "caption"
    ));
    assert!(matches!(
        &typed.parts[1],
        OutboundContentPart::Content { part: ContentPart::Image { media, .. } }
            if media.url.as_deref() == Some("https://example.test/image.png")
    ));
}

#[test]
fn each_legacy_payload_variant_adapts_to_its_matching_typed_part() {
    let cases = [
        (
            OutboundPayload::Voice {
                media_url: "https://example.test/voice.ogg".into(),
            },
            1,
        ),
        (
            OutboundPayload::Files {
                file_urls: vec!["https://example.test/file.pdf".into()],
            },
            1,
        ),
        (
            OutboundPayload::Poll {
                question: "Pick one".into(),
                options: vec!["A".into(), "B".into()],
            },
            1,
        ),
        (
            OutboundPayload::PresentationBlocks {
                blocks: serde_json::json!([{"type": "section"}]),
            },
            1,
        ),
        (
            OutboundPayload::NativeChannelData {
                data: serde_json::json!({"content": "native"}),
            },
            1,
        ),
        (
            OutboundPayload::Media {
                text: None,
                media_urls: vec![],
            },
            0,
        ),
    ];

    for (payload, expected_parts) in cases {
        let typed = ChannelOutboundMessage::from_intent(&ChannelOutboundIntent {
            channel_id: "telegram".into(),
            conversation_id: "fallback-room".into(),
            payload,
            ..Default::default()
        });
        assert_eq!(typed.parts.len(), expected_parts);
    }
}

#[test]
fn typed_text_projection_preserves_reference_addressing_and_rejects_formatted_text() {
    let intent = ChannelOutboundIntent {
        channel_ref: Some(ChannelRef {
            id: "telegram:account".into(),
            account_id: None,
        }),
        conversation_ref: Some(ConversationRef {
            id: "ref-room".into(),
            thread_id: Some("ref-thread".into()),
            ..Default::default()
        }),
        thread_id: None,
        idempotency_key: "key".into(),
        payload: OutboundPayload::Text {
            text: "hello".into(),
        },
        ..Default::default()
    };
    let typed = ChannelOutboundMessage::from_intent(&intent);
    assert_eq!(typed.channel_id, "telegram:account");
    assert_eq!(typed.conversation_id, "ref-room");
    assert_eq!(typed.thread_id.as_deref(), Some("ref-thread"));
    let legacy = typed.to_legacy_message().unwrap();
    assert_eq!(legacy.recipient, "ref-room");
    assert_eq!(legacy.thread_ts.as_deref(), Some("ref-thread"));
    assert_eq!(legacy.idempotency_key.as_deref(), Some("key"));

    let formatted = ChannelOutboundMessage {
        parts: vec![OutboundContentPart::Content {
            part: ContentPart::Text {
                text: "hello".into(),
                markdown_dialect: Some("commonmark".into()),
            },
        }],
        ..typed
    };
    assert!(formatted.to_legacy_message().is_err());
}
