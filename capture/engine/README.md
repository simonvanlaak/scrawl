# Staged capture Engine

This independently compiled Rust package implements admission, durable receipts and
bundle maintenance exclusion. **No Desktop, timed recording, Agent or retention
call path consumes it yet.** It does not record media, publish clipboard content,
route captures, or certify a complete Apparent bundle. Linux process tests are
real; macOS compilation and process behavior remain unverified.

## Caller contract

- Provision one absolute, current-user-owned `0700` runtime directory on a local
  filesystem supporting hard links, advisory locks, atomic rename and fsync.
  Every product must use the SAME persistent directory. Do not place it inside
  a bundle, a disposable runtime/temp tree, or a renamed deployment directory.
  Do not delete/rotate it during upgrade, restart or routine cleanup.
- Generate and retain `new_id()` request and bundle identities and the separate
  `new_owner_token()` **before the first attempt**. Retrying changes neither.
  IDs are 128-bit random lowercase hex; mutation authority is a separate random
  256-bit token. Neither IDs nor process IDs authorize Stop/Cancel.
- `Runtime::admit(request, token, bundle)` returns `Acquired(Session)` only after
  durable admission and kernel-held global recording, per-request owner and
  shared bundle locks. Begin ALL media side effects after this return, never
  before it. `Existing` means inspect the durable receipt; never start again.
  Busy admissions persist a `Rejected` tombstone. A later intentional new
  capture needs a fresh request ID, not retry of that tombstone.
- Keep `Session` alive through adapter startup, recording, postprocessing and
  finalizer completion. `recording()` marks actual first-frame readiness;
  `processing()` follows recording. Do not report readiness just because a
  thread/process started. Startup failure calls `finish(Err(()))`.
- `request_control(request, token, Stop|Cancel)` records authenticated intent.
  The owner polls `Session::control()` and stops its own adapter safely. Cancel
  is sticky, can supersede Stop, and prevents Saved even when a proof arrives.
  Controls never signal a PID or claim the media has already stopped. Caller
  timeout does NOT cancel; no clock/TTL can steal a live owner's locks.
- Only after the real finalizer validates all required media/artifacts and
  completes its durability barrier, call `FinalizerSuccess::validate` with the
  exact request, bundle, absolute final directory and manifest bytes. It checks
  the manifest identity and exact bytes, rejects unfinished/unsafe-video markers
  and pins directory identity. `Session::finish(Ok(proof))` rechecks that proof
  and session binding before Saved. It is NOT a general bundle/media validator;
  its JSON fixture in tests is deliberately synthetic, not production media.
- A dropped/killed owner never manufactures success. `inspect` (or an authorized
  retry) recovers an abandoned active receipt to Failed, or Cancelled if intent
  was already persisted. Recovery is based on the kernel owner lock, not a PID.
  Terminal receipts survive normal exit/SIGKILL and never restart or change on
  later controls. A crash after successful finalization but before terminal
  publication remains Failed, not speculative Saved.
- Acquire `lease(bundle_id, Read|Write|Finalize)` for every relevant reader,
  writer and finalizer. All three are SHARED maintenance exclusions, not writer
  serialization. `Maintenance` is EXCLUSIVE and returns `None` on contention;
  keep it through rename/delete/shrink/recovery mutation. Request IDs do not
  identify bundle locks. Reuse the persisted bundle ID across pathname changes;
  never generate a new ID to clean up an already-known bundle.

All locks use maintained Rust 1.89 `File` locking, with narrow Unix no-follow
filesystem operations. They are close-on-exec. A raw fork inherits lock-holding
file descriptions; leaking one conservatively keeps admission/cleanup blocked
until the LAST inherited descriptor closes. Do not fork-and-continue a recorder;
use exec or explicitly close child copies. Process tests cover both cases.

Lock names and hard-link identity anchors are permanent. The Engine never
unlinks lock files. Unsafe modes, symlinks, replaced lock inodes, malformed or
unknown-version receipts and replaced runtime roots fail closed. An interrupted
lock/anchor bootstrap also fails closed rather than guessing a repair. Repair
requires separately proving every consumer stopped and retaining receipts;
there is intentionally no automatic stale-lock deletion API. Private `0600`
record files contain authority; public receipts and errors do not expose it.
This is cooperative same-user coordination, NOT a sandbox against malicious
code already able to rewrite both runtime state and its identity anchors.

## Integration work intentionally left to the parent

Wire the common runtime into Desktop/shortcut `Recording::start` paths,
`src/main.rs` timed recording and `src/agent_capture.rs` before any media starts.
Replace neither a process-local Registry nor the socket listener lock with an
unsupported global claim. Poll authenticated intent in the owning adapter,
keep the guard in its real finalization lifetime, and supply that finalizer's
validated bytes. Wire bundle leases into `src/retention.rs`, `src/recover.rs`,
export/reprocess/read paths before removing process-local BUSY/RECORDING guards.
Audit all call paths together; adding this package alone provides no whole-app
exclusion or production adoption.

## Verification and package boundary

Run from the repository root:

```
cargo test --manifest-path capture/engine/Cargo.toml --offline
cargo build --manifest-path capture/engine/Cargo.toml --offline
cargo fmt --manifest-path capture/engine/Cargo.toml -- --check
cargo tree --manifest-path capture/engine/Cargo.toml --offline
```

The package pins cached libc/serde/serde_json dependencies and has its own small,
unmodified Cargo-generated lockfile. Root Cargo.toml/Cargo.lock and CI are
unchanged: this is a staged package, not a final workspace/lock policy decision.
There are no Agent, GPUI, recorder or product dependencies. Repository hooks and
the 2,000-physical-line-per-file cap still apply. Native macOS compilation,
filesystem/crash tests and both products' real integration are outstanding gates.
