# Disposable Linux GUI tests

The Omarchy VM is a *source* for an offline template, not a shared test desktop. Each GUI job gets a qcow2 copy-on-write overlay, its own OVMF variables, loopback-only SSH forwarding, Unix VNC/QMP sockets, and a fresh SSH host-key cache. The candidate binary and its sibling shared libraries are copied into that guest and hash-checked. The scenario writes evidence under `$SCRAWL_GUI_OUTPUT`; the runner copies it to the named result directory and deletes the writable disk after QEMU exits. No job starts/stops Apparent in the live Omarchy guest. Docker remains suitable for `cargo test` and non-GUI checks, but a nested container compositor would not validate the existing Hyprland/portal path without separate proof.

## One-time template preparation

**Only after coordinating a quiet maintenance handoff.** Never copy or `qemu-img -U` the live guest's disk while QEMU is running, and never stop another worker's capture. Verify no active capture or Apparent process, shut down the guest cleanly, then check QEMU has exited and its disk is unlocked. Preserve the original VM and its launcher. First make a private staging copy of the stopped image. In that copy, replace the test user's home with the distribution's `/etc/skel` (including captures, drafts, browser profiles, agent accounts and autostart), install a separate test-only SSH key, clear logs and network connections, rotate the guest SSH host key and machine ID, and remove old Btrfs snapshots. Do not clean the original guest. Give the private test-only account a desktop autologin; SSH and the SDDM greeter alone are not a ready desktop. Shut down the staging guest and make the immutable template **from the sanitized, stopped staging copy**:

```sh
python3 scripts/linux/prepare-gui-vm.py \
  --source "$SANITIZED_STAGING_DISK" \
  --ovmf-vars "$SANITIZED_STAGING_OVMF_VARS" \
  --destination ~/VMs/scrawl-gui-template
qemu-img info ~/VMs/scrawl-gui-template/system.qcow2
```

The preparation command refuses a source referenced by a running QEMU process, and checks again after conversion. It does not coordinate with the launcher: keep the source VM stopped throughout the copy. A clean shutdown, rather than an external live qcow2 copy, is the consistency boundary.

Do not include private captures, unsent drafts, credentials, or a running Apparent service in a template offered to multiple agent jobs. The local template is `~/VMs/scrawl-gui-template` and its test-only SSH identity lives alongside it, outside the repository. The original VM is separate. A fresh, empty machine ID is regenerated on each overlay boot; SSH host keys are test-only and the per-job SSH ports and known-hosts files are isolated. Keep all forwarding loopback-only. Inspect the guest home and Btrfs subvolumes before promoting a new template: removing visible files alone does not remove snapshot references. Do not infer capture readiness from the greeter's Hyprland process.

## Run a job

From this repository checkout, after the offline Rust tests and a fresh `cargo build`, package the exact candidate with `./scripts/linux/appimage.sh target/debug/apparent`, then run `python3 -B scripts/linux/gui-test.py update`. The short CLI selects the private local template and creates a fresh results directory. Do not use an AppImage from another checkout. The guest needs `grim`, `ffmpeg` and `tesseract`.

This job stages and hash-checks the *package*, launches it with `APPIMAGE_EXTRACT_AND_RUN=1` (so FUSE is not required), checks that the active process came from that AppImage, opens its real Settings window, clicks About and **Check for Updates…** over the job's private QMP socket, captures the checking state while a fixture holds `curl`, and then captures no-update, installed-update and error results. The VM blocks outbound connections while retaining loopback-only SSH forwarding; the fixture responds to exact GitHub update URLs inside the disposable guest, and an available update swaps only a private copy of the staged AppImage. The runner writes `result.json`, `scenario.log`, screenshots, focused OCR crops, fixture requests and the installed-update sidecar to its private results directory. Read those images and the scenario log, not just the exit code; this does not prove a live release feed or a real network download. The original VM and X1 are untouched. A package hash and `--help` alone are not GUI evidence.

Use `--binary /absolute/path/to/Apparent-x86_64.AppImage` for another checkout's freshly packaged candidate; never infer that this checkout's package contains edits made elsewhere. The command prints its private results path. For full control, call the underlying runner:

```sh
python3 scripts/linux/gui-vm.py \
  --template ~/VMs/scrawl-gui-template/system.qcow2 \
  --ovmf-vars ~/VMs/scrawl-gui-template/OVMF_VARS.fd \
  --ssh-user scrawl --ssh-key ~/VMs/scrawl-gui-template/test_access \
  --binary target/appimage/Apparent-x86_64.AppImage \
  --scenario scripts/linux/gui-update-settings.sh \
  --qmp-input --isolated-network \
  --results "$TMPDIR/apparent-update-gui-$(date +%s)"
```

The runner refuses a live or chained base image and never edits it. A scenario failure propagates as the command exit code; QEMU is stopped and the overlay removed even on failure. The test VM boots cold for now: read `boot_seconds` from real runs before deciding whether a safe RAM snapshot is worth the complexity. Jobs can run concurrently while RAM/disk permit. Do not point two jobs at the live guest's writable disk. Test runner contract locally with `python3 -m unittest discover -s scripts/linux -p test_gui_vm.py -v`.

**RAM and abandoned jobs:** Each disposable QEMU is launched in its own process group. Normal completion, errors and SIGTERM stop that group, wait for QEMU, then delete only that job's writable disk and firmware. Before boot, the runner also arms a separate **user-systemd timer** (15 minutes by default, `--job-timeout SECONDS` to change it). It refuses to boot if the timer cannot be armed. A job-local copy of the reaper survives removal of this checkout. If the runner is SIGKILLed or loses its agent session, the timer stops QEMU for that **one result directory** and removes its writable state only after QEMU no longer owns it; logs and evidence remain. It does not stop the original VM or another job. The timer is canceled on normal cleanup and garbage-collected after expiry. A hard timeout ends a long-running capture, so increase it deliberately for a longer scenario. If the host/user systemd manager is itself down, no user-space timer can run; a host reboot then frees RAM, but an unclean job disk must be checked before removal. The original Omarchy VM is intentionally long-lived and separate; its RAM remains allocated while it runs.

The default is 3072 MiB per guest. Check the *worker's cgroup* memory limit as well as host `free`: a QEMU process can be killed by its individual worker scope even when the host still has plenty of RAM. Two simultaneous 3072 MiB capture jobs passed on this host; that is not an admission guarantee for arbitrary parallel counts. Use a larger `--memory` only after checking the limit. To inspect currently armed expiries, run `systemctl --user list-timers 'scrawl-gui-expire-*.timer'`; check QEMU process arguments for an exact job disk before manual cleanup. Never remove the template or another job's overlay.
