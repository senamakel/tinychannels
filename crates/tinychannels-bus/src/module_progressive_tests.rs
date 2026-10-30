use super::SubagentOutput;

#[test]
fn subagent_output_keeps_display_label_and_detail() {
    let output = SubagentOutput {
        display_label: Some("Review changes".into()),
        display_detail: Some("Inspecting the channel adapter".into()),
        ..Default::default()
    };
    let value = serde_json::to_value(&output).unwrap();
    assert_eq!(value["displayLabel"], "Review changes");
    assert_eq!(value["displayDetail"], "Inspecting the channel adapter");
    assert_eq!(
        serde_json::from_value::<SubagentOutput>(value).unwrap(),
        output
    );
}
