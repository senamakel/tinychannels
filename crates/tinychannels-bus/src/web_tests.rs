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

#[test]
fn compatibility_web_contract_round_trips_request_metadata_and_error_details() {
    let metadata = ChatRequestMetadata {
        source: Some("ptt".into()),
        speak_reply: Some(true),
        session_id: Some(93),
        agent_id: Some("agent-a".into()),
        hold_text_stream: true,
        ..Default::default()
    };
    let encoded = serde_json::to_value(&metadata).unwrap();
    assert_eq!(encoded["speakReply"], true);
    assert_eq!(encoded["sessionId"], 93);
    assert_eq!(
        serde_json::from_value::<ChatRequestMetadata>(encoded).unwrap(),
        metadata
    );

    let event = WebChannelEvent::ChatError {
        context: WebEventContext::default(),
        message: Some("blocked".into()),
        error_type: Some("guardrail".into()),
        error_source: Some("prompt_guard".into()),
        error_retryable: Some(false),
        error_retry_after_ms: None,
        error_provider: None,
        error_fallback_available: Some(false),
        copy_key: Some("chat.error.blocked".into()),
        copy_params: None,
        guardrail: Some(GuardrailPayload {
            verdict: "block".into(),
            score: 0.9,
            reasons: vec![GuardrailReason {
                code: "prompt_injection".into(),
                message: "blocked".into(),
            }],
        }),
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["error_source"], "prompt_guard");
    assert_eq!(json["guardrail"]["reasons"][0]["code"], "prompt_injection");
    assert_eq!(
        serde_json::from_value::<WebChannelEvent>(json).unwrap(),
        event
    );
}

#[test]
fn timing_snapshot_tracks_first_tool_and_round_one_narration() {
    let mut state = TurnTimingState::default();
    state.text_delta(12, "hello", 1);
    state.text_delta(20, " again", 2);
    state.tool_call(32);
    state.tool_call(44);
    let snapshot = state.snapshot(1_000);
    assert_eq!(snapshot.first_token_ms, Some(12));
    assert_eq!(snapshot.first_tool_ms, Some(32));
    assert_eq!(snapshot.round_one_narration_chars, 5);
    assert_eq!(snapshot.tool_calls, 2);

    let payload = snapshot.into_payload(Some(40));
    assert_eq!(payload.elapsed_ms, 1_000);
    assert_eq!(payload.tokens_per_second, Some(40.0));
    assert_eq!(payload.first_tool_ms, Some(32));
}

#[test]
fn projected_tool_call_omits_empty_name_and_null_arguments() {
    let mut event = WebChannelEvent::ToolCall {
        round: None,
        skill_id: None,
        tool_display_label: None,
        tool_display_detail: None,
        context: WebEventContext::default(),
        tool_name: Some(String::new()),
        tool_call_id: Some("call-1".into()),
        args: Some(Value::Null),
    };
    event.normalize_projected_wire_fields();
    let wire = serde_json::to_value(event).unwrap();
    assert!(wire.get("tool_name").is_none());
    assert!(wire.get("args").is_none());
}

#[test]
fn absent_web_event_fields_are_omitted_and_present_false_values_survive() {
    for name in [
        "chat_error",
        "artifact_pending",
        "artifact_ready",
        "artifact_failed",
        "approval_request",
        "plan_review_decided",
    ] {
        let input = serde_json::json!({
            "event": name, "client_id": "client", "thread_id": "thread", "request_id": "request"
        });
        let event: WebChannelEvent = serde_json::from_value(input).unwrap();
        let wire = serde_json::to_value(&event).unwrap();
        assert!(
            wire.as_object()
                .unwrap()
                .values()
                .all(|value| !value.is_null()),
            "{name} must omit absent optional fields: {wire}"
        );
        assert_eq!(
            serde_json::from_value::<WebChannelEvent>(wire).unwrap(),
            event
        );
    }
    for (name, field) in [
        ("approval_request", "detached"),
        ("chat_error", "error_fallback_available"),
    ] {
        let input = serde_json::json!({
            "event": name, "client_id": "client", "thread_id": "thread", "request_id": "request",
            field: false, "seq": 0, "ts": 0
        });
        let event: WebChannelEvent = serde_json::from_value(input).unwrap();
        let wire = serde_json::to_value(&event).unwrap();
        assert_eq!(wire[field], false);
        assert_eq!(wire["seq"], 0);
        assert_eq!(wire["ts"], 0);
        assert_eq!(
            serde_json::from_value::<WebChannelEvent>(wire).unwrap(),
            event
        );
    }
}
