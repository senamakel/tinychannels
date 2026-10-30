use super::*;

#[test]
fn tool_payload_caps_are_utf8_safe_and_bounded() {
    let capped = cap_web_output("🦀".repeat(MAX_WEB_TOOL_PAYLOAD_BYTES));
    assert!(capped.len() <= MAX_WEB_TOOL_PAYLOAD_BYTES);
    assert!(capped.ends_with("… [output truncated]"));
    assert!(cap_web_args(Some(serde_json::json!({"ok": true}))).is_some());
}

#[test]
fn web_event_round_trip_preserves_addressing_context() {
    let event = WebChannelEvent::TextDelta {
        context: WebEventContext {
            client_id: "c".into(),
            request_id: "r".into(),
            ..Default::default()
        },
        delta: Some("hello".into()),
        delta_kind: Some("text".into()),
        round: Some(1),
    };
    assert_eq!(event.event_name(), "text_delta");
    assert_eq!(event.context().request_id, "r");
    assert_eq!(
        serde_json::from_slice::<WebChannelEvent>(&serde_json::to_vec(&event).unwrap()).unwrap(),
        event
    );
}
