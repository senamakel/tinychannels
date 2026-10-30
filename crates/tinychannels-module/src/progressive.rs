//! Bounded module-owned progressive reply workers and retained terminal cleanup.
use crate::host::HostChannels;
use async_trait::async_trait;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tinybus::{Error, Result};
use tinychannels::delivery::progressive::{
    EDIT_FLUSH_INTERVAL, FILLER_INTERVAL, ProgressiveReply, ProgressiveSendError,
    ProgressiveSender, TYPING_REFRESH_INTERVAL,
};
use tinychannels_bus::module_progressive::{
    ChannelOutputEvent, ProgressiveCommand, ProgressiveEvent, ProgressiveOutputFailure,
    ProgressiveResult, ProgressiveTransportRequest, ProgressiveTransportResult,
};
use tinychannels_bus::{ChannelBackendRequest, ChannelBackendResult};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;

const MAX_ACTIVE: usize = 16;
const MAX_RETAINED: usize = 128;
const MAX_CONTENT: usize = 384 * 1024;
// Both text and thinking can fill their content budget in one feed. JSON
// escaping expands a byte to at most six bytes; retain bounded envelope space.
const MAX_REQUEST: usize = MAX_CONTENT * 12 + 64 * 1024;
const TURN_LIFETIME: Duration = Duration::from_secs(180);
const MAX_TYPED_OUTPUT_LIFETIME: Duration = Duration::from_secs(30 * 60);

struct Sender(HostChannels);

impl Sender {
    async fn transport(
        &self,
        request: ProgressiveTransportRequest,
    ) -> std::result::Result<ProgressiveTransportResult, ProgressiveSendError> {
        let result = self
            .0
            .dispatch_controller_backend(ChannelBackendRequest::ProgressiveTransport(request))
            .await
            .map_err(|_| ProgressiveSendError::Unavailable)?;
        match result {
            ChannelBackendResult::ProgressiveTransport(result) => Ok(result),
            _ => Err(ProgressiveSendError::Other(anyhow::anyhow!(
                "progressive transport returned an invalid result"
            ))),
        }
    }

    async fn invoke(
        &self,
        request: ProgressiveTransportRequest,
    ) -> std::result::Result<Value, ProgressiveSendError> {
        let result = self
            .0
            .dispatch_controller_backend(ChannelBackendRequest::ProgressiveTransport(request))
            .await
            .map_err(|_| ProgressiveSendError::Unavailable)?;
        match result {
            ChannelBackendResult::ProgressiveTransport(ProgressiveTransportResult::Success(
                value,
            )) => Ok(value),
            ChannelBackendResult::ProgressiveTransport(ProgressiveTransportResult::Unavailable) => {
                Err(ProgressiveSendError::Unavailable)
            }
            ChannelBackendResult::ProgressiveTransport(
                ProgressiveTransportResult::EditUnsupported,
            ) => Err(ProgressiveSendError::EditUnsupported),
            ChannelBackendResult::ProgressiveTransport(ProgressiveTransportResult::MessageGone) => {
                Err(ProgressiveSendError::MessageGone)
            }
            _ => Err(ProgressiveSendError::Other(anyhow::anyhow!(
                "progressive transport failed"
            ))),
        }
    }
}

#[async_trait]
impl ProgressiveSender for Sender {
    async fn send(
        &self,
        channel: &str,
        text: &str,
    ) -> std::result::Result<Value, ProgressiveSendError> {
        let intent = tinychannels::outbound_intent_from_legacy_message(
            channel,
            serde_json::json!({"text": text}),
        );
        let message = tinychannels::legacy_message_value_from_outbound_intent(&intent);
        self.invoke(ProgressiveTransportRequest::Send {
            channel: channel.into(),
            turn_id: None,
            channel_ref: None,
            conversation: None,
            idempotency_key: None,
            message,
        })
        .await
    }
    async fn edit(
        &self,
        channel: &str,
        message_id: &str,
        text: &str,
    ) -> std::result::Result<(), ProgressiveSendError> {
        self.invoke(ProgressiveTransportRequest::Edit {
            channel: channel.into(),
            turn_id: None,
            channel_ref: None,
            conversation: None,
            message_id: message_id.into(),
            receipt: None,
            text: text.into(),
        })
        .await
        .map(drop)
    }
    async fn delete(
        &self,
        channel: &str,
        message_id: &str,
    ) -> std::result::Result<(), ProgressiveSendError> {
        self.invoke(ProgressiveTransportRequest::Delete {
            channel: channel.into(),
            channel_ref: None,
            conversation: None,
            receipt: None,
            message_id: message_id.into(),
        })
        .await
        .map(drop)
    }
    async fn typing(&self, channel: &str) -> std::result::Result<(), ProgressiveSendError> {
        self.invoke(ProgressiveTransportRequest::Typing {
            channel: channel.into(),
            conversation: None,
        })
        .await
        .map(drop)
    }
}

enum Command {
    Feed(Vec<ProgressiveEvent>, oneshot::Sender<Result<()>>),
    Finish(String, oneshot::Sender<()>),
}

struct Closing {
    fingerprint: [u8; 32],
    task: Option<JoinHandle<std::result::Result<(), String>>>,
    outcome: Option<std::result::Result<(), String>>,
}
struct Entry {
    commands: mpsc::Sender<Command>,
    worker: Mutex<Option<JoinHandle<()>>>,
    closing: Mutex<Option<Closing>>,
}
struct TypedOutputState {
    next_sequence: u64,
}
struct TypedOutputEntry {
    channel: tinychannels_bus::ChannelRef,
    conversation: tinychannels_bus::ConversationRef,
    turn_id: String,
    hosted: bool,
    expires_at: std::time::Instant,
    state: Mutex<TypedOutputState>,
}
struct State {
    terminal: bool,
    entries: HashMap<String, Arc<Entry>>,
    typed_entries: HashMap<String, Arc<TypedOutputEntry>>,
}

/// One private runtime's reply handles; all callbacks retain that runtime's host.
pub(crate) struct Replies {
    host: HostChannels,
    state: Mutex<State>,
}

impl Replies {
    pub(crate) fn new(host: HostChannels) -> Self {
        Self {
            host,
            state: Mutex::new(State {
                terminal: false,
                entries: HashMap::new(),
                typed_entries: HashMap::new(),
            }),
        }
    }

    pub(crate) async fn execute(&self, request: ProgressiveCommand) -> Result<ProgressiveResult> {
        if serde_json::to_vec(&request).map_or(true, |bytes| bytes.len() > MAX_REQUEST) {
            return Err(Error::failed("progressive request exceeds its limit"));
        }
        match request {
            ProgressiveCommand::Open { channel } => self.open(channel).await,
            ProgressiveCommand::Feed { handle, events } => {
                if events.len() > 32 {
                    return Err(Error::failed("too many progressive events"));
                }
                let entry = self.entry(&handle).await?;
                let closing = entry.closing.lock().await;
                if closing.is_some() {
                    return Err(Error::failed("progressive reply is closed"));
                }
                let (reply, wait) = oneshot::channel();
                entry
                    .commands
                    .send(Command::Feed(events, reply))
                    .await
                    .map_err(|_| Error::failed("progressive reply is closed"))?;
                drop(closing);
                wait.await
                    .map_err(|_| Error::failed("progressive worker stopped"))??;
                Ok(ProgressiveResult::Updated)
            }
            ProgressiveCommand::Finish { handle, text } => {
                if text.len() > MAX_CONTENT {
                    return Err(Error::failed("progressive content exceeds its limit"));
                }
                let entry = self.entry(&handle).await?;
                close_entry(entry, text, false).await?;
                Ok(ProgressiveResult::Closed)
            }
            ProgressiveCommand::Close { handle } => {
                let entry = {
                    let mut state = self.state.lock().await;
                    if state.typed_entries.remove(&handle).is_some() {
                        return Ok(ProgressiveResult::Closed);
                    }
                    state.entries.get(&handle).cloned()
                };
                if let Some(entry) = entry {
                    close_entry(entry, "Cancelled.".into(), true).await?;
                }
                Ok(ProgressiveResult::Closed)
            }
            ProgressiveCommand::OpenOutput {
                channel,
                conversation,
                turn_id,
                lifetime_ms,
            } => {
                self.open_typed(channel, conversation, turn_id, lifetime_ms, false)
                    .await
            }
            ProgressiveCommand::OpenHostedOutput {
                channel,
                conversation,
                turn_id,
                lifetime_ms,
            } => {
                self.open_typed(channel, conversation, turn_id, lifetime_ms, true)
                    .await
            }
            ProgressiveCommand::FeedOutput {
                handle,
                turn_id,
                channel_ref,
                conversation,
                sequence,
                events,
            } => {
                self.feed_output(
                    &handle,
                    turn_id.as_deref(),
                    channel_ref.as_ref(),
                    conversation.as_ref(),
                    sequence,
                    events,
                )
                .await
            }
        }
    }

    async fn open_typed(
        &self,
        channel: tinychannels_bus::ChannelRef,
        conversation: tinychannels_bus::ConversationRef,
        turn_id: String,
        lifetime_ms: u64,
        hosted: bool,
    ) -> Result<ProgressiveResult> {
        if channel.id.trim().is_empty()
            || channel.id.len() > 128
            || conversation.id.trim().is_empty()
            || conversation.id.len() > 4096
            || turn_id.trim().is_empty()
            || turn_id.len() > 128
        {
            return Err(Error::failed("invalid progressive output scope"));
        }
        if let Some(account) = &channel.account_id {
            if account.trim().is_empty() || account.len() > 256 {
                return Err(Error::failed("invalid progressive output channel account"));
            }
        }
        if lifetime_ms == 0 || Duration::from_millis(lifetime_ms) > MAX_TYPED_OUTPUT_LIFETIME {
            return Err(Error::failed(
                "progressive output lifetime is outside its limit",
            ));
        }

        let mut state = self.state.lock().await;
        if state.terminal {
            return Err(Error::failed("progressive replies are shutting down"));
        }
        let now = std::time::Instant::now();
        state
            .typed_entries
            .retain(|_, entry| entry.expires_at > now);
        if state.typed_entries.len() >= MAX_ACTIVE {
            return Err(Error::failed("progressive output capacity reached"));
        }
        let handle = uuid::Uuid::new_v4().simple().to_string();
        state.typed_entries.insert(
            handle.clone(),
            Arc::new(TypedOutputEntry {
                channel,
                conversation,
                turn_id,
                hosted,
                expires_at: now + Duration::from_millis(lifetime_ms),
                state: Mutex::new(TypedOutputState { next_sequence: 0 }),
            }),
        );
        Ok(ProgressiveResult::Opened { handle })
    }

    async fn feed_output(
        &self,
        handle: &str,
        turn_id: Option<&str>,
        channel_ref: Option<&tinychannels_bus::ChannelRef>,
        conversation: Option<&tinychannels_bus::ConversationRef>,
        sequence: Option<u64>,
        events: Vec<ChannelOutputEvent>,
    ) -> Result<ProgressiveResult> {
        if events.len() > 32 {
            return Err(Error::failed("too many progressive output events"));
        }
        let entry = {
            let state = self.state.lock().await;
            if state.terminal {
                return Err(Error::failed("progressive replies are shutting down"));
            }
            state
                .typed_entries
                .get(handle)
                .cloned()
                .ok_or_else(|| Error::failed("unknown progressive output handle"))?
        };
        if entry.expires_at <= std::time::Instant::now() {
            return Err(Error::failed("progressive output handle expired"));
        }
        if turn_id.is_some_and(|turn_id| turn_id != entry.turn_id)
            || channel_ref.is_some_and(|channel| channel != &entry.channel)
            || conversation.is_some_and(|conversation| conversation != &entry.conversation)
        {
            return Err(Error::failed("progressive output scope mismatch"));
        }
        let request_bytes = serde_json::to_vec(&events)
            .map_err(|_| Error::failed("invalid progressive output events"))?;
        if request_bytes.len() > MAX_REQUEST {
            return Err(Error::failed(
                "progressive output events exceed their limit",
            ));
        }

        let mut typed_state = entry.state.lock().await;
        if let Some(sequence) = sequence {
            if sequence < typed_state.next_sequence {
                return Err(Error::failed("progressive output sequence is stale"));
            }
            typed_state.next_sequence = sequence;
        }
        let sender = Sender(self.host.clone());
        let mut outcomes = Vec::new();
        let mut hosted_accepted = false;
        let mut failure = None;
        for event in events {
            let event_sequence = typed_state.next_sequence;
            typed_state.next_sequence = typed_state
                .next_sequence
                .checked_add(1)
                .ok_or_else(|| Error::failed("progressive output sequence exhausted"))?;
            let observed = sender
                .transport(ProgressiveTransportRequest::OutputEvent {
                    channel: entry.channel.id.clone(),
                    channel_ref: Some(entry.channel.clone()),
                    conversation: Some(entry.conversation.clone()),
                    turn_id: entry.turn_id.clone(),
                    sequence: event_sequence,
                    event: Box::new(event.clone()),
                })
                .await;
            let accepted = match observed {
                Ok(ProgressiveTransportResult::Acknowledged) => false,
                Ok(ProgressiveTransportResult::Accepted) if entry.hosted => true,
                Ok(ProgressiveTransportResult::Accepted) => {
                    failure = Some(ProgressiveOutputFailure::ProviderRejected);
                    continue;
                }
                Ok(ProgressiveTransportResult::Unavailable) => {
                    failure = Some(ProgressiveOutputFailure::HostUnavailable);
                    continue;
                }
                Ok(_) | Err(_) => {
                    failure = Some(ProgressiveOutputFailure::ProviderTransport);
                    continue;
                }
            };
            if let Some((parts, text)) = outbound_delivery(&event) {
                if accepted {
                    hosted_accepted = true;
                    continue;
                }
                if entry.hosted {
                    failure = Some(ProgressiveOutputFailure::ProviderRejected);
                    continue;
                }
                let sent = sender
                    .transport(ProgressiveTransportRequest::SendTyped {
                        channel: entry.channel.id.clone(),
                        turn_id: Some(entry.turn_id.clone()),
                        channel_ref: Some(entry.channel.clone()),
                        conversation: Some(entry.conversation.clone()),
                        sequence: Some(event_sequence),
                        segment_index: Some(0),
                        idempotency_key: Some(format!("{}:{event_sequence}:0", entry.turn_id)),
                        parts,
                        text,
                    })
                    .await;
                match sent {
                    Ok(ProgressiveTransportResult::Outcome(outcome)) => outcomes.push(*outcome),
                    Ok(ProgressiveTransportResult::Accepted) if entry.hosted => {
                        hosted_accepted = true;
                    }
                    Ok(ProgressiveTransportResult::Unavailable) => {
                        failure = Some(ProgressiveOutputFailure::HostUnavailable);
                    }
                    Ok(_) | Err(_) => {
                        failure = Some(ProgressiveOutputFailure::ProviderTransport);
                    }
                }
            }
        }
        if hosted_accepted || failure.is_some() {
            return Ok(ProgressiveResult::DeliveryBatch {
                outcomes,
                unsupported: Vec::new(),
                failure,
                hosted_accepted,
            });
        }
        match outcomes.len() {
            0 => Ok(ProgressiveResult::Updated),
            1 => Ok(ProgressiveResult::DeliveryOutcome {
                outcome: Box::new(outcomes.pop().expect("one delivery outcome")),
            }),
            _ => Ok(ProgressiveResult::DeliveryOutcomes { outcomes }),
        }
    }

    async fn entry(&self, handle: &str) -> Result<Arc<Entry>> {
        let state = self.state.lock().await;
        if state.terminal {
            return Err(Error::failed("progressive replies are shutting down"));
        }
        state
            .entries
            .get(handle)
            .cloned()
            .ok_or_else(|| Error::failed("unknown progressive handle"))
    }

    async fn open(&self, channel: String) -> Result<ProgressiveResult> {
        if channel.is_empty() || channel.len() > 4096 {
            return Err(Error::failed("invalid progressive target"));
        }
        let mut state = self.state.lock().await;
        if state.terminal {
            return Err(Error::failed("progressive replies are shutting down"));
        }
        let mut completed = Vec::new();
        let mut active = 0;
        for (handle, entry) in &state.entries {
            let finished = match entry.worker.try_lock() {
                Ok(worker) => match worker.as_ref() {
                    Some(worker) => worker.is_finished(),
                    None => match entry.closing.try_lock() {
                        Ok(closing) => closing.as_ref().is_some_and(|closing| {
                            closing.outcome.is_some()
                                || closing.task.as_ref().is_some_and(|task| task.is_finished())
                        }),
                        Err(_) => false,
                    },
                },
                Err(_) => false,
            };
            if finished {
                completed.push(handle.clone());
            } else {
                active += 1;
            }
        }
        if active >= MAX_ACTIVE {
            return Err(Error::failed("progressive reply capacity reached"));
        }
        if state.entries.len() >= MAX_RETAINED {
            if let Some(handle) = completed.into_iter().next() {
                state.entries.remove(&handle);
            } else {
                return Err(Error::failed("progressive reply capacity reached"));
            }
        }
        let handle = uuid::Uuid::new_v4().simple().to_string();
        let (commands, inbox) = mpsc::channel(16);
        let host = self.host.clone();
        let worker = tokio::spawn(run(host, channel, inbox));
        state.entries.insert(
            handle.clone(),
            Arc::new(Entry {
                commands,
                worker: Mutex::new(Some(worker)),
                closing: Mutex::new(None),
            }),
        );
        Ok(ProgressiveResult::Opened { handle })
    }

    /// Freeze admission and await terminal delivery before callback shutdown.
    pub(crate) async fn shutdown(&self) -> Result<()> {
        let entries = {
            let mut state = self.state.lock().await;
            state.terminal = true;
            state.typed_entries.clear();
            state.entries.values().cloned().collect::<Vec<_>>()
        };
        // Start every terminal task before awaiting any one provider. A blocked
        // delivery must not prevent unrelated reply workers from draining.
        let mut closes = tokio::task::JoinSet::new();
        for entry in entries {
            closes.spawn(close_entry(entry, "Cancelled.".into(), true));
        }
        let mut failure = None;
        while let Some(result) = closes.join_next().await {
            let result =
                result.unwrap_or_else(|_| Err(Error::failed("progressive shutdown worker failed")));
            if let Err(error) = result {
                failure.get_or_insert(error);
            }
        }
        if let Some(error) = failure {
            // Retained per-entry terminal tasks can finish after the timeout;
            // the next Shutdown awaits them without sending a second reply.
            return Err(error);
        }
        let mut state = self.state.lock().await;
        state.entries.clear();
        state.typed_entries.clear();
        Ok(())
    }
}

fn outbound_delivery(
    event: &ChannelOutputEvent,
) -> Option<(Vec<tinychannels_bus::OutboundContentPart>, String)> {
    use tinychannels_bus::{ContentPart, OutboundContentPart, OutboundVoice};
    let text_part = |text: &str| OutboundContentPart::Content {
        part: ContentPart::Text {
            text: text.to_owned(),
            markdown_dialect: None,
        },
    };
    match event {
        ChannelOutputEvent::FinalMessage { text, parts, .. } => {
            let parts = if parts.is_empty() {
                vec![text_part(text)]
            } else {
                parts
                    .iter()
                    .cloned()
                    .map(|part| OutboundContentPart::Content { part })
                    .collect()
            };
            Some((parts, text.clone()))
        }
        ChannelOutputEvent::Media { text, parts } => {
            let mut outbound: Vec<_> = text.iter().map(|text| text_part(text)).collect();
            outbound.extend(
                parts
                    .iter()
                    .cloned()
                    .map(|part| OutboundContentPart::Content { part }),
            );
            Some((outbound, text.clone().unwrap_or_default()))
        }
        ChannelOutputEvent::VoiceNote { text, voice } => {
            let text = text.clone().or_else(|| {
                voice
                    .transcript
                    .as_ref()
                    .map(|transcript| transcript.text.clone())
            });
            let mut outbound: Vec<_> = text.iter().map(|text| text_part(text)).collect();
            outbound.push(OutboundContentPart::Voice {
                voice: OutboundVoice::Media {
                    media: voice.media.clone(),
                    codec: voice.codec.clone(),
                    duration_ms: voice.duration_ms,
                },
            });
            Some((outbound, text.unwrap_or_default()))
        }
        _ => None,
    }
}

async fn close_entry(entry: Arc<Entry>, text: String, retain_existing: bool) -> Result<()> {
    let fingerprint: [u8; 32] = Sha256::digest(text.as_bytes()).into();
    let mut closing = entry.closing.lock().await;
    if let Some(existing) = closing.as_ref() {
        if !retain_existing && existing.fingerprint != fingerprint {
            return Err(Error::failed("progressive terminal content differs"));
        }
    } else {
        let retained = entry.clone();
        let task = tokio::spawn(async move {
            let (reply, wait) = oneshot::channel();
            if retained
                .commands
                .send(Command::Finish(text, reply))
                .await
                .is_ok()
            {
                let _ = wait.await;
            }
            if let Some(worker) = retained.worker.lock().await.take() {
                worker
                    .await
                    .map_err(|_| "progressive worker failed".to_string())?;
            }
            Ok(())
        });
        *closing = Some(Closing {
            fingerprint,
            task: Some(task),
            outcome: None,
        });
    }
    let closing = closing.as_mut().expect("terminal work was installed");
    if closing.outcome.is_none() {
        let task = closing
            .task
            .as_mut()
            .expect("unfinished terminal task is retained");
        let result = tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .map_err(|_| Error::failed("progressive cleanup is pending"))?
            .map_err(|_| Error::failed("progressive cleanup worker failed"))?;
        closing.outcome = Some(result);
        closing.task = None;
    }
    closing
        .outcome
        .clone()
        .expect("terminal result was retained")
        .map_err(|_| Error::failed("progressive cleanup failed"))
}

async fn run(host: HostChannels, channel: String, mut inbox: mpsc::Receiver<Command>) {
    let mut reply = ProgressiveReply::new(Arc::new(Sender(host)), channel);
    let mut edits = tokio::time::interval(EDIT_FLUSH_INTERVAL);
    let mut typing = tokio::time::interval(TYPING_REFRESH_INTERVAL);
    let mut fillers = tokio::time::interval(FILLER_INTERVAL);
    for timer in [&mut edits, &mut typing, &mut fillers] {
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        timer.tick().await;
    }
    reply.typing_tick().await;
    let expiry = tokio::time::sleep(TURN_LIFETIME);
    tokio::pin!(expiry);
    loop {
        tokio::select! {
            command = inbox.recv() => match command {
                Some(Command::Feed(events, response)) => {
                    let result = apply(&mut reply, events);
                    let _ = response.send(result);
                }
                Some(Command::Finish(text, response)) => {
                    reply.finalize(&text).await;
                    let _ = response.send(());
                    break;
                }
                None => { reply.finalize("Cancelled.").await; break; }
            },
            _ = edits.tick() => reply.edit_tick().await,
            _ = typing.tick() => reply.typing_tick().await,
            _ = fillers.tick() => reply.filler_tick().await,
            _ = &mut expiry => { reply.finalize("Sorry, the request timed out.").await; break; }
        }
    }
}

fn apply(reply: &mut ProgressiveReply, events: Vec<ProgressiveEvent>) -> Result<()> {
    let text = events
        .iter()
        .filter_map(|event| match event {
            ProgressiveEvent::TextDelta(value) => Some(value.len()),
            _ => None,
        })
        .sum::<usize>();
    let thinking = events
        .iter()
        .filter_map(|event| match event {
            ProgressiveEvent::ThinkingDelta(value) => Some(value.len()),
            _ => None,
        })
        .sum::<usize>();
    if reply.state().content.len().saturating_add(text) > MAX_CONTENT
        || reply
            .state()
            .thinking_accumulator
            .len()
            .saturating_add(thinking)
            > MAX_CONTENT
    {
        return Err(Error::failed("progressive content exceeds its limit"));
    }
    for event in events {
        match event {
            ProgressiveEvent::TextDelta(value) => reply.on_text_delta(&value),
            ProgressiveEvent::ThinkingDelta(value) => reply.on_thinking_delta(&value),
            ProgressiveEvent::ToolCall(name) => reply.on_tool_call(&name),
            ProgressiveEvent::ToolResult { tool_name, success } => {
                reply.on_tool_result(&tool_name, success)
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "progressive_tests.rs"]
mod tests;
