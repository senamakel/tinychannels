# Runtime driver test draft (not executed)

This source checkpoint contains an API-absence compile test and a live,
unattested-peer refusal behavior test in `src/lib_runtime_tests.rs`. They were
not run because Cargo is reserved by Juice. The compile test is expected to
fail until `ChannelRuntimeDriver`, `TinyChannelsClient::runtime_driver()`, and
`runtime_driver_with_dispatch()` are implemented. No RED or GREEN result is
claimed.

The positive A/B and lifecycle cases still need an executable fixture. It must
load a locally staged, digest-pinned native artifact through
`ModuleRuntime::host()` and `ModuleRuntime::connection()` and use the same
`ModuleRuntime` for `lease()`. Do not use `Broker::attest_module`, another
broker for the callback, or raw TinyBus transport in the product API.

For a controllable A/B test, the native A fixture can pause Reserve, release
its alias, and accept lifecycle calls. Then the test can claim B through
`ModuleRuntime::serve_host_object()` on that same broker. That helper only
serves ordinary, unattested host objects, so it cannot be used as a
confidential-call target or as a confidential negative-control canary. Label
B as an ordinary, unattested availability control: prove an ordinary call
through the alias reaches B, and prove a positive typed object call still
returns A's distinct response after handover. B's zero-call count alone does
not prove that a bound call stayed on A. If a test needs a live B to accept a
confidential colliding-handle canary, B must be a genuine admitted native
artifact or use a supported public `attach_linked` path with actual
attestation; never manufacture attestation.

Keep a separate native composition test for the actual `tinychannels-module`
cdylib with an empty provider configuration. It can prove real
ReserveRuntime, returned-object, start, object-call, and STOP ACK behavior
without provider or external network access. Claim callback traffic only
after observing an actual module-originated callback on the same runtime.
The controllable fixture's callback test does not stand in for this production
module proof.

Once source and Cargo access are granted, add the executable cases in order:

1. Bind A strictly before gated Reserve; verify exact handle and object path,
   callback registration/traffic, Start, positive typed A object result, and
   ACK STOP all stay on A after the alias moves to B. Keep the ordinary B
   availability check separate and explicitly unattested.
2. Exercise confidential root lifecycle calls and each confidential typed
   object wrapper; verify ordinary object methods remain ordinary. Include the
   explicit StartRuntimeWithDispatch choice and the `StartRelay` response as
   `tinychannels_bus::relay::CapabilityDescriptor`.
3. Assert opaque Start errors become `StartOutcome::Unacknowledged`, opaque
   STOP errors become `StopOutcome::Unacknowledged`, and an error string such
   as “cleanup pending” never enables another STOP. Only a decoded STOP ACK
   withdraws the exact callback.
4. Cover malformed reservation/object responses, single sanitized reporting,
   and lease cancellation before Reserve reply, during object/start, and at
   caller close timeout. Keep the native real-module proof separate from
   client-projection fixture results.
