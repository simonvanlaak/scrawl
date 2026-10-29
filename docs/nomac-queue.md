# NoMac queue proof gate

`scripts/nomac-wait.py` is the source for the separately installed `nomac-wait` operator CLI. Test it without rentals:

```sh
python3 -m unittest discover -s scripts/tests -p 'test_nomac_wait.py'
```

## Pin before allocating

From the original Git checkout root, inspect the candidate and intended test phase offline. Check the PR's live head/base and prior proof ledger too. A queue ticket is not approval to rent for whatever happens to be in that folder.

```sh
nomac-wait enqueue task-unique-ticket
nomac-wait candidate
# Review and record that fingerprint, the exact remote base SHA, and test plan.
nomac-wait pin task-unique-ticket \
  --candidate <reviewed-64-character-fingerprint> \
  --base-sha <reviewed-full-base-commit-sha> --phase final
nomac-wait wait task-unique-ticket --slice 400 --poll 15 \
  --max-duration 1800 --idle-timeout 300 --max-cost 0.5
```

`pin` requires explicit full values; it never silently approves the current tree. Use `--phase diagnostic` for an authorized baseline investigation, not a final production-candidate test. The default base is `origin` / `refs/heads/main`; another configured remote name and full branch ref can be selected with `--remote` and `--base-ref` on `pin`.

New tickets wait **one hour by default**; both `enqueue` and implicit ticket creation by `wait` refuse `--deadline-hours` below 1. An explicit longer deadline is allowed up to 24 hours. Repeating `wait` on an existing ticket keeps its original deadline; it does not reset the hour, and legacy tickets with shorter recorded deadlines are not silently extended or discarded. `--slice 400` is only one foreground tool-call slice (terminal timeout 590 seconds), not a shorter queue timeout. In T3, keep one foreground waiter active in the original thread and retry exit 75 with the same ticket inside the same turn when the candidate remains eligible; do not detach the command or create a scheduled retry merely to display Waiting. This shows **Working**, not Waiting. Inspect an expiry or safety error rather than opening a new ticket blindly.

The fingerprint includes HEAD, all tracked working-tree files (including deletions), all non-ignored untracked files, executable bits, and symlink targets. This accepts an exact uncommitted candidate, including staged/unstaged changes and new files. It hashes local bytes without saving or printing a diff, file names, or file contents. Ignored build products, external symlink contents, toolchains and other dependencies are **not** source identity. Submodules and non-file entries are refused. Do not use ignored files as candidate source. Keep the checkout frozen through transfer and verify the same reviewed bytes on the Mac before testing.

Immediately before **each** provider `start`, while holding the existing queue lock, the CLI checks the source fingerprint and live remote base SHA, then rechecks the source after the network lookup. Any base advance fails closed, even if only docs changed: review relevance offline and explicitly repin the same ticket when appropriate. A failed check leaves the queued record and FIFO position intact and makes no NoMac call. Git errors are redacted rather than printing credential-bearing URLs. Proof verification has a 15-second budget; network failure refuses allocation, not a fallback to stale refs. The existing 590-second foreground timeout still allows a 400-second slice, 15-second preflight, 165-second start and five-second lock wait under ordinary local filesystem operation.

This is an identity-drift guard, **not** proof that the candidate was reviewed, the PR still targets this head, or an earlier Mac test did not already cover it. It cannot prevent a concurrent edit or remote push after the final check. Maintain the operator preflight and proof ledger; do not edit a checkout while a waiter can allocate from it.

## Existing tickets and rollout

There is no automatic migration or installation. Review and install the candidate CLI separately before using it against the shared queue. Stop the exact scheduled/foreground execution owner before changing its executable; read back the schedule state. Do not run an old allocator beside the new one: an older binary does not know about this gate.

An existing **queued** ticket without `proof` remains in place, but cannot allocate until its owner reviews and explicitly pins it from its original checkout. Do not cancel/re-enqueue it to migrate: that loses FIFO identity. Repinning preserves its deadline, creation time, workspace, request key and backoff. A stale unpinned head ticket continues to block later tickets; this is deliberate, not permission to skip it.

`starting`, `provisioning` and `acquired` records retain their existing recovery, inspection and cleanup paths even when they have no proof or source moved. They may own a billable Mac. They cannot be repinned. If recovery returns an uncertain start to `queued`, the proof gate applies before its next idempotent start; first reconcile possible ownership, then explicitly pin if needed. Never delete a possible session to get past the gate. Ready-session output is access to an owned session, not a claim that current source has Mac proof.

## One execution owner

The queue lock serializes CLI operations and prevents simultaneous starts. It does not identify T3 scheduler tasks or distinguish a scheduled tick from a foreground turn in the same checkout. Adding a caller-supplied owner label would not prove that the other execution route stopped, and a persistent lease would complicate interruption/recovery without disabling its schedule. Keep this as operator policy: exactly one foreground waiter **or** one bounded scheduled retry per ticket; disable and read back the exact schedule before switching. Reuse the ticket, never create a second ticket to switch modes.
