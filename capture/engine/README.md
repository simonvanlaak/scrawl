# Staged capture Engine

This independently compiled Rust package implements admission, durable receipts and
bundle maintenance exclusion. Desktop screen/phone capture, timed CLI capture,
Agent requests using the GUI coordinator and retention now consume it through
`src/capture_runtime`. The package itself does not record media, publish clipboard
content, route captures or certify a complete Apparent bundle. Linux host/process
tests are not native GUI proof; macOS compilation and native behavior remain gates.

## Caller contract

- Provision one absolute, current-user-owned `0700` runtime directory on a local
  filesystem supporting hard links, advisory locks, atomic rename and fsync.
  Every product must use the SAME persistent directory. Do not place it inside
  a bundle, a disposable runtime/temp tree, or a renamed deployment directory.
  Do not delete/rotate it during upgrade, restart or routine cleanup.
- Generate and retain the `new_id()` request identity and separate
  `new_owner_token()` **before the first attempt**. Retrying changes neither.
  Request IDs remain 128-bit random lowercase hex; mutation authority remains a
  separate random 256-bit token. Neither bundle IDs nor PIDs authorize controls.
  Reuse the bundle's persisted identity: `apparent-<milliseconds>-<hex-pid>`
  (including its counter suffix), caller-selected output stems, and legacy
  random hex IDs are supported. A bundle identity is 1–255 UTF-8 bytes, excluding
  control characters, `/`, `\`, `.` and `..`. Unicode is not normalized or case
  folded. Retain the exact manifest ID through reads, recovery and path renames.
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
- Output trust is stricter than no-follow: the absolute output path must have
  no symlinks and every ancestor must belong to root or the effective user and
  disallow group/other writes. Only root-owned sticky ancestors (such as `/tmp`)
  are exempt from the write-mode rule; their owned children remain protected.
  The final directory must belong to the effective user and disallow group/other
  writes, without exception. The proof pins that directory and checks both its
  descriptor and freshly traversed path at construction and again before Saved.
  The manifest must be an owned, single-link, non-group/other-writable regular
  file. ACLs, privileged attackers and malicious same-user mutation are outside
  this mode-bit/cooperative boundary; callers must provision output roots without
  ACL grants to other writers. Validation never repairs unsafe modes or owners.
- Manifests have a separate **64 MiB** byte budget, not the runtime receipt's
  64 KiB budget. `src/capture.rs::write_bundle_artifacts` serializes the entire
  `ApparentBundle` with pretty JSON, including unbounded event/stroke-point,
  transcript and context histories (`src/model.rs`); 64 KiB is not realistic.
  The Engine rejects supplied bytes above 64 MiB before parsing/copying, reads
  at most 64 MiB + 1 from the pinned manifest and requires exact bytes. This is
  a bounded integration budget, not a promise every possible capture fits. A
  production caller must surface over-budget finalization as failure or revise
  the budget/representation with measured evidence before wiring; never truncate.
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

Legacy 32-lowerhex bundle IDs retain their original `bundle-<id>.lock` anchors.
Other bundle IDs use `bundle-v1/<hex-chunks>/identity.lock` under the runtime
root: exact UTF-8 bytes encoded as lowercase hex, in 120-character chunks.
This is reversible and injective, including prefix IDs and Unicode equivalents;
no digest collision, path interpolation or per-operation random mapping exists.
Namespace directories are permanent owned `0700` directories, opened relative
to pinned descriptors with no-follow, fsynced and rechecked with their parents.
Never rename/delete these directories independently of the runtime lock anchors.

## Adopted callers and inspectable runtime

`src/capture_runtime/mod.rs` admits before media startup, retains the session
through synchronous source teardown and detached finalization, and consumes exact
checked publication bytes before returning success. `src/ui/recording.rs` uses
this for both source types, including Agent requests dispatched through that GUI
coordinator. `src/main.rs` timed screen/phone captures use the same adapter.
Readiness is the real producer acknowledgement read after join; a synthetic
fallback frame cannot advance the Engine to Recording. Retention trash/shrink
uses Engine `Maintenance` exclusion before its existing process-local claim.

Production opens one persistent `dirs::data_local_dir()/apparent/capture-runtime-v1`:

- Linux: `${XDG_DATA_HOME:-$HOME/.local/share}/apparent/capture-runtime-v1`.
- macOS: `~/Library/Application Support/apparent/capture-runtime-v1`.

This is runtime authority, not a preference store or disposable cache. Inspect
path ownership/modes without changing them. Use `Runtime::inspect(request_id)`
for a sanitized receipt rather than displaying raw `request-*.json` records,
which contain private owner authority. `inspect` can reconcile an abandoned
receipt using its owner lock; it is not merely a read of JSON. There is no new
Engine-inspection CLI here, and `apparent inspect` inspects capture bundles, not
these receipts. Do not publish raw records, owner tokens or private bundle data.

Never unlink, rotate or replace lock files, identity anchors or namespace
directories, even if the named process is gone or a receipt looks old. Preserve
the runtime directory across restart/upgrade and preserve receipts. Do not chmod
an unsafe existing tree to bypass rejection. Recovery/repair requires separately
proving every consumer has stopped; stale age or PID checks are not that proof.

Each timed CLI capture generates a fresh internal Engine request ID and separate
owner token. Those are not legacy socket/wire capture IDs or bundle IDs. Existing
in-process compatibility receipts and socket listener locks remain; they confer
no Engine control authority and are not a cross-process receipt API.

This is scoped adoption, not whole-app exclusion. Recovery, export/reprocess and
other reader/writer lease coverage still need a separate audit before removing
process-local BUSY/RECORDING guards. Native platform acceptance and affected caller
review remain separate from package tests or provider approval.

## Verification and package boundary

Run from the repository root:

```
cargo test --manifest-path capture/engine/Cargo.toml --offline
cargo build --manifest-path capture/engine/Cargo.toml --offline
cargo fmt --manifest-path capture/engine/Cargo.toml -- --check
cargo tree --manifest-path capture/engine/Cargo.toml --offline
```

The package pins cached libc/serde/serde_json dependencies and has its own small,
unmodified Cargo-generated lockfile. Root Cargo.toml/Cargo.lock now declare and
resolve this path dependency on macOS and Linux; CI is unchanged. This is package
integration plus the scoped caller adoption above, not a standalone Agent.
There are no Agent, GPUI, recorder or product dependencies inside the Engine.
Repository hooks still apply: recognized dependency lockfile basenames are
exempt from the 2,000-physical-line cap; other text remains capped. See
`../../.githooks/README.md` for the exact names. Native macOS compilation,
filesystem/crash tests and native caller integration remain outstanding gates.

Tests that construct finalizer proofs need a synthetic `TMPDIR` with trusted
ancestors under the output contract above. A group-writable cache ancestor is
correctly rejected even when the individual fixture is private; do not chmod a
real cache or capture root just to make a test pass.
