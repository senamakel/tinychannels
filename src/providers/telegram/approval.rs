//! Telegram approval-prompt vocabulary shared by hosts.

/// Identifier used for Telegram-originated approval contexts.
pub const TELEGRAM_APPROVAL_CLIENT_ID: &str = "telegram";

/// Render an approval request as a Telegram message body.
pub fn format_approval_prompt(tool_name: &str, action_summary: &str) -> String {
    format!(
        "🔐 Approval needed\nTool: `{tool_name}`\nAction: {action_summary}\n\nReply `yes` to approve or `no` to deny."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_prompt_includes_action_and_reply_instructions() {
        let body = format_approval_prompt("git_operations", "git commit -m fix");
        assert!(body.contains("git_operations"));
        assert!(body.contains("git commit"));
        assert!(body.contains("yes") && body.contains("no"));
    }
}
