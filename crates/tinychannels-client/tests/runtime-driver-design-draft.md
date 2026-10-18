# Runtime driver verification scope

The typed API and source tests now exist. Cargo has not been run on the
implementation because Search owns the current Cargo window. No GREEN or
native-proven behavior is claimed yet.

The A/B, lifecycle, uncertainty and typed-object cases are written in
`tests/runtime_native_e2e.rs`. Its support fixture loads a locally staged,
digest-pinned native artifact through `ModuleRuntime::host()` and
`ModuleRuntime::connection()` and uses that same `ModuleRuntime` for
`lease()`. Do not use `Broker::attest_module`, another broker for the callback,
or raw TinyBus transport in the product API.

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

The separate `tests/native_module_runtime_e2e.rs` test loads the actual
`tinychannels-module` cdylib with an empty provider configuration. It exercises
real ReserveRuntime, returned-object, start, typed object-call and STOP ACK
behavior without provider or external network access. Its callback assertion
probes the registered host callback from the host connection; it does not
claim module-originated callback traffic. The controllable fixture's callback
traffic assertion is separate from this production module composition proof.

Remaining tests include malformed reservation/object responses with one
sanitized report and caller cancellation before Reserve reply, during
object/start, and at close timeout. The real module composition and its
provider-free limits remain separate from the controllable fixture results.
