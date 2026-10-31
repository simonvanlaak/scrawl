# Checked bundle publication

The production pipeline returns an error when required persistence, validation,
recovery-marker cleanup or durability fails. This is a persistence bug fix,
not a new bundle format. This layer alone creates no Engine receipt or Saved
claim; the adopted callers below consume its checked result.

## Production entry points and caller ordering

`finalize_capture_bundle` and `build_bundle_from_parts` use
`bundle_pipeline::publication::publish_after(path, build)`. Existing tuple/result
contracts remain intact. This checks both prepared PNGs and rendered images.
`recover::write` and `speech_model::mark_pending` now return checked results;
the pipeline propagates them instead of discarding persistence failures.

The **outermost** `publish_after` must enclose all optional whiteboard mutations
and any fallback. Nested builds on that thread defer their barrier to that outer
operation. The outer return is the only success eligible for downstream effects.
An incomplete base build retains `.publication-pending` and recovery work.

Both GUI source branches in `src/ui/recording.rs` now use
`finalize_owned_capture`. It keeps `whiteboard::finish_handoff`, including its
fallback, inside `Capture::publish` in `src/capture_runtime/mod.rs`. That adapter
holds an outer coordination lease across `publish_after`, `checked`, Engine
validation and terminal handling. Only its successful result reaches
`receive_capture_result` and the existing success effects. Merely checking after
a whiteboard mutation would not preserve this ordering: a pending writer could
already have started.

Timed CLI screen/phone and GUI capture acquire Engine admission before starting
media and retain ownership through source teardown and detached finalization.
Final real-frame readiness comes from the stopped, joined sampler/phone reader,
not a pre-stop snapshot or synthetic candidate. Intentional phone stop suppresses
unsolicited notifications before joining; queued EOF events carry the originating
session identity and cannot end a replacement preview or recording.

Host regressions exercise the real finalization/dispatch seams with success-effect
spies; they do not execute a desktop clipboard write or hosted upload. Native
macOS screen/phone/helper and Linux GUI/capture/preview-switch proof remain gates.
See `capture/engine/README.md` for the runtime path and remaining lease coverage.

## Engine inputs and writer lease

`coordinate(path) -> Result<Coordination>` is a non-Send, owning-thread reentrant
writer lease over a persistent `.publication.lock` flock inode. Never unlink that
file. Hold an outer lease across final mutation, `checked`, and Engine consumption.
Reentrant guards share the locked descriptor and original directory identity.
They can drop in either order: coordination lasts until the last guard drops.
The `Rc` ownership makes guards neither `Send` nor `Sync`; acquire a new lease
on another thread instead of moving or sharing a guard. Nested `publish_after`
calls defer to the outer build, and a pending worker waits for all its guards.
`checked(path, &bundle, &summary) -> Result<PublicationInputs>` returns the
absolute `path`, `bundle_id`, and exact persisted `manifest` bytes. Feed those,
plus the Engine caller's own request identity, to existing
`FinalizerSuccess::validate` and `Session::finish`; this layer creates neither.
A root test exercises the existing Engine validator without calling `finish`.
Engine additionally rejects group-writable ancestors even below a private parent;
provision a path satisfying its existing contract, not the shared build scratch.

The lease serializes cooperative finalizers and pending transcript writers, not
arbitrary same-user processes or subsequent retention/reprocess work. The inputs
are not a perpetual media attestation. `Capture::publish` retains its outer lease
between `checked` and Engine terminal handling; future callers must do the same.

## Persistence and validation

Descriptor-relative Unix operations reject symlinks at managed ancestors/leaves,
traversal, nonregular files, hard links, untrusted owners and world-writable
artifacts. Bundle roots are private; existing group-writable owned artifacts are
made private before their validation snapshot. Inventory, relative paths, JSON,
PNG allocation and movie metadata/sample counts are bounded. Required references
include keyframes, contact sheets and both autocrop resolutions. Present retained
PNG/WAV/movie files are checked too. Image-only bundles do not require a movie or
WAV; phone/device bundles use the same reference-based rules.

PNG pixels are decoded with the existing `png` crate through stream completion.
WAV structure and every sample are checked with `hound`. MP4/MOV checks cover
completed ISO-BMFF boxes, embedded video descriptions, sample sizes, chunk extents
and timing counts. **Movie codec payload decoding is not claimed**; fragmented
MP4 and unusual sample-table variants are not supported by this validator. No
optional platform decoder, new dependency or external ffmpeg process is required
at publication. Tiny checked-in H.264 MP4 and genuine QuickTime MOV fixtures
are synthetic (see their README for muxing, sample tables and full-frame decode
evidence). They do not establish compatibility with every native writer variant.

All inventoried files, nested directories, bundle root and parent are fsynced;
metadata, inode identity and the complete inventory are checked for replacement.
Recovery records disappear only after artifact validation and fsync. Cleanup
preflights all markers and restores the original markers if final cleanup or
identity/durability checks fail, reporting restoration errors rather than success.
No filesystem can promise recovery when it also refuses those restoration writes.

Pending transcription is marked durably without spawning an in-place writer.
A model-ready scan or successful outer publication resumes it under the same
lease. Incomplete/unsafe/journaled bundles are skipped with pending work intact.
Transcription errors retain pending work; successful updates pass the barrier
before marker removal. A failed update attempts to restore and revalidate the
previous manifest/summary so a later retry is not stranded.

## Proof limits

Offline Linux root-package build/tests use synthetic artifacts only. Native
macOS/Linux GUI, codec playback, real crash/power-loss and forced fsync-failure
proof remain separate. No capture, VM, install, deploy, live config or Engine
terminal integration is authorized or claimed by these tests.
