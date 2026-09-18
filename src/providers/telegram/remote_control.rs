//! Portable Telegram remote-control command vocabulary and rendering.

/// Telegram command that is handled by the host's remote-control adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramRemoteCommand {
    Status,
    Sessions,
    New,
    Help,
}

/// Maximum number of sessions displayed by the standard `/sessions` response.
pub const SESSIONS_LIST_LIMIT: usize = 8;

/// Parse a Telegram remote-control command, accepting bot mentions and case
/// differences in the same way as Telegram's command surface.
pub fn parse_telegram_remote_command(content: &str) -> Option<TelegramRemoteCommand> {
    let command = content.trim().split_whitespace().next()?;
    let command = command
        .strip_prefix('/')?
        .split('@')
        .next()
        .unwrap_or(command)
        .to_ascii_lowercase();

    match command.as_str() {
        "status" => Some(TelegramRemoteCommand::Status),
        "sessions" => Some(TelegramRemoteCommand::Sessions),
        "new" => Some(TelegramRemoteCommand::New),
        "help" => Some(TelegramRemoteCommand::Help),
        _ => None,
    }
}

/// Render the portable help text for Telegram remote control.
pub fn build_remote_help_response() -> String {
    [
        "OpenHuman Telegram remote control (phase 1):",
        "",
        "• `/status` — active thread, model, and turn state",
        "• `/sessions` — recent conversation threads",
        "• `/new` — start a fresh thread for this chat",
        "• `/help` — this message",
        "",
        "Model routing: `/model`, `/models` (same as before).",
    ]
    .join("\n")
}

/// Render a session row for a Telegram `/sessions` response.
pub fn format_session_line(title: &str, id: &str, message_count: usize, active: bool) -> String {
    let marker = if active { "→ " } else { "  " };
    let title = if title.trim().is_empty() { id } else { title };
    format!("{marker}`{title}` — {message_count} msgs (id: `{id}`)")
}

/// Render the successful `/new` response after the host creates and binds a
/// conversation thread.
pub fn build_new_session_response(title: &str, thread_id: &str) -> String {
    format!(
        "Started new session **{title}**.\nThread id: `{thread_id}`\nIn-memory channel history cleared for this chat."
    )
}

/// Render the standard `/status` response from host-supplied state.
pub fn build_status_response(
    thread_line: &str,
    provider: &str,
    model: &str,
    history_len: usize,
    busy: bool,
) -> String {
    let turn_state = if busy { "in progress ⏳" } else { "idle" };
    format!(
        "**Status**\n{thread_line}\nProvider: `{provider}`\nModel: `{model}`\nIn-memory turns: {history_len}\nTurn: {turn_state}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands_and_renders_responses() {
        assert_eq!(
            parse_telegram_remote_command(" /STATUS@OpenHumanBot now "),
            Some(TelegramRemoteCommand::Status)
        );
        assert_eq!(
            parse_telegram_remote_command("/sessions"),
            Some(TelegramRemoteCommand::Sessions)
        );
        assert!(parse_telegram_remote_command("/model").is_none());

        assert!(build_remote_help_response().contains("`/status`"));
        assert!(format_session_line("", "thread-1", 2, true).starts_with("→ `thread-1`"));
        assert!(build_new_session_response("Today", "thread-1").contains("thread-1"));
        assert!(
            build_status_response("Thread: none", "openai", "gpt-5", 3, true)
                .contains("in progress")
        );
    }
}
