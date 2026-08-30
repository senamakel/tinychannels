# tinychannels-bus

The TinyBus **wire contract** for TinyChannels: the call vocabulary a host and
the channels module share, with none of the machinery that speaks it.

This crate is deliberately transport-free and dependency-light. It owns the
types that cross the boundary — inbound envelopes, outbound intents, channel
configuration, controller metadata, relay frames, the pairing helpers and the
session-key rules — and nothing that opens a socket, spawns a task or touches a
database. The provider stack (Telegram, Discord, Slack, IMAP, WhatsApp, …) and
the relay transport loop live in the [`tinychannels`](../..) crate.

## Which crate to depend on

| You need to… | Depend on |
| --- | --- |
| *name* a channel type — an envelope in an event enum, the config schema, session-key derivation | `tinychannels-bus` |
| actually connect to a provider | `tinychannels` (which re-exports this crate whole) |

The split matters because the second one pulls `reqwest`, `rusqlite`, `rustls`
and `tokio-tungstenite`. A host that only decodes payloads should not carry a
provider stack to do it.

## Rules

- **Never re-declare a contract type in a host.** A field added on one side of a
  copy is a decode failure on the other, with nothing to catch it.
- **Call members by their constant**, never a string literal —
  `methods::SEND_MESSAGE`, not `"SendMessage"`. A rename upstream is then a
  compile error rather than a `MemberNotFound` in the field.
- **Host policy stays host-side.** This crate says what may be *sent*; it does
  not decide what a host will act on.

## Two objects, opposite directions

A channel is bidirectional, and a served object cannot open a stream back to its
caller. So the module serves `BUS_NAME` (the host drives providers through it)
and the **host** serves `HOST_BUS_NAME` (the module delivers what arrives from
the network through that). Both are declared in [`src/names.rs`](src/names.rs).

## Two in-process seams

Everything here is transport-free except two items, and the exception is
documented rather than averaged away: `traits::Channel` hands a provider a
`tokio::sync::mpsc::Sender`, and `security` runs its constant-time pairing
compare on the blocking pool. The `tokio` dependency is pinned to `sync` + `rt`
for exactly that reason — no scheduler, no I/O driver, no timers. Anything
needing a wider tokio feature belongs in `tinychannels`.
