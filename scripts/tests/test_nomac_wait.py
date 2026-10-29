"""Queue acquisition without renting a Mac; the fake CLI implements start/status."""
import hashlib
import json
import os
import runpy
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "nomac-wait.py"


class NoMacWaitTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.work = self.root / "work"
        self.work.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        (self.work / "source").write_text("baseline\n")
        self.git("add", "source")
        self.git("commit", "-m", "baseline")
        self.base = self.git("rev-parse", "HEAD").strip()
        self.remote = self.root / "remote.git"
        self.git("clone", "--bare", str(self.work), str(self.remote))
        self.git("remote", "add", "origin", str(self.remote))
        self.state = self.root / "state"
        self.home = self.root / "home"
        self.home.mkdir()
        self.mode = self.root / "mode"
        self.mode.write_text("capacity")
        self.calls = self.root / "calls"
        self.nomac = self.root / "nomac"
        self.nomac.write_text("#!/usr/bin/env python3\n"
            "import hashlib, json, os, pathlib, sys, time\n"
            "pathlib.Path(os.environ['FAKE_CALLS']).open('a').write(' '.join(sys.argv[1:])+'\\n')\n"
            "mode=pathlib.Path(os.environ['FAKE_MODE']).read_text()\n"
            "if mode=='slow-capacity' and sys.argv[1]=='start': time.sleep(7); mode='capacity'\n"
            "if sys.argv[1]=='status':\n"
            " if mode=='no-session': print(json.dumps({'session': None})); sys.exit(0)\n"
            " print(json.dumps({'session': {'id': 'ses_test', 'state': mode, 'cleanup_confirmed': mode=='stopped'}})); sys.exit(0)\n"
            "key=sys.argv[sys.argv.index('--request-key')+1]\n"
            "name=hashlib.sha256(str(pathlib.Path.cwd()).encode()).hexdigest()[:24]\n"
            "local=pathlib.Path.home()/'.nomac'/'sessions'/name/'session.json'\n"
            "local.parent.mkdir(parents=True, exist_ok=True)\n"
            "local.write_text(json.dumps({'mode':'hosted', 'requestKey':key, 'id': None if mode in ('capacity','failed','unknown') else 'ses_test'}))\n"
            "if mode=='capacity':\n"
            " print(json.dumps({'error': {'code': 'session_limit_reached'}}), file=sys.stderr); sys.exit(1)\n"
            "if mode=='failed':\n"
            " print(json.dumps({'error': {'code': 'insufficient_credit'}}), file=sys.stderr); sys.exit(1)\n"
            "if mode=='poststart':\n"
            " print(json.dumps({'error': {'code': 'ssh_setup_failed'}}), file=sys.stderr); sys.exit(1)\n"
            "if mode=='unknown':\n"
            " print('unknown response'); sys.exit(0)\n"
            "print(json.dumps({'session': {'id': 'ses_test', 'state': mode}}))\n")
        self.nomac.chmod(0o700)

    def git(self, *args):
        env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        result = subprocess.run(["git", *args], cwd=self.work, env=env,
                                text=True, capture_output=True, check=True)
        return result.stdout

    def pin_ticket(self, ticket):
        candidate = self.raw_cli("candidate")
        self.assertEqual(candidate.returncode, 0, candidate.stderr)
        result = self.raw_cli("pin", ticket, "--candidate", candidate.stdout.strip(),
                              "--base-sha", self.base, "--phase", "final")
        self.assertEqual(result.returncode, 0, result.stderr)

    def run_cli(self, *args):
        # Queue-contract fixtures approve a candidate once, not on every wait.
        # Gate and migration tests use raw_cli without automatic approval.
        if args[0] in ("enqueue", "wait") and not (self.state / (args[1] + ".json")).exists():
            deadline = args[args.index("--deadline-hours") + 1] if "--deadline-hours" in args else "1"
            result = self.raw_cli("enqueue", args[1], "--deadline-hours", deadline)
            if result.returncode:
                return result
            self.pin_ticket(args[1])
            if args[0] == "enqueue":
                return result
        return self.raw_cli(*args)

    def raw_cli(self, *args):
        env = dict(os.environ, HOME=str(self.home), FAKE_CALLS=str(self.calls), FAKE_MODE=str(self.mode))
        return subprocess.run([sys.executable, str(SCRIPT), "--state-dir", str(self.state),
                               "--nomac", str(self.nomac), *args], cwd=self.work,
                              env=env, text=True, capture_output=True, timeout=10)

    def test_capacity_is_quiet_and_resumes_same_ticket(self):
        self.assertEqual(self.run_cli("enqueue", "ticket-a").returncode, 0)
        first_key = json.loads((self.state / "ticket-a.json").read_text())["request_key"]
        first = self.run_cli("wait", "ticket-a", "--slice", "0", "--poll", "300")
        self.assertEqual(first.returncode, 75, first.stderr)
        self.assertEqual(first.stdout, "")
        self.assertEqual(len(self.calls.read_text().splitlines()), 1)
        second = self.run_cli("wait", "ticket-a", "--slice", "0", "--poll", "300")
        self.assertEqual(second.returncode, 75, second.stderr)
        self.assertEqual(len(self.calls.read_text().splitlines()), 1)
        self.mode.write_text("ready")
        record = self.state / "ticket-a.json"
        data = json.loads(record.read_text())
        self.assertNotEqual(data["request_key"], first_key)
        data["next_attempt"] = 0
        record.write_text(json.dumps(data))
        result = self.run_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["session_id"], "ses_test")

    def test_legacy_queued_ticket_requires_pin_without_losing_fifo(self):
        self.assertEqual(self.raw_cli("enqueue", "ticket-a").returncode, 0)
        path = self.state / "ticket-a.json"
        before = path.read_bytes()
        self.mode.write_text("ready")
        result = self.raw_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("pin", result.stderr)
        self.assertEqual(path.read_bytes(), before)
        self.assertFalse(self.calls.exists())

    def test_legacy_pin_preserves_fifo_and_ticket_identity(self):
        self.raw_cli("enqueue", "ticket-a")
        path = self.state / "ticket-a.json"
        original = json.loads(path.read_text())
        self.raw_cli("enqueue", "ticket-b")
        self.pin_ticket("ticket-b")
        self.mode.write_text("ready")
        self.assertEqual(self.raw_cli("wait", "ticket-b", "--slice", "0").returncode, 75)
        self.assertFalse(self.calls.exists())
        self.pin_ticket("ticket-a")
        after = json.loads(path.read_text())
        self.assertEqual({key: after[key] for key in original}, original)
        self.assertEqual(self.raw_cli("wait", "ticket-a", "--slice", "0").returncode, 0)

    def test_pinned_dirty_candidate_is_allowed_but_changed_bytes_are_not(self):
        (self.work / "source").write_text("private changed source\n")
        (self.work / "new-file").write_text("private untracked source\n")
        self.run_cli("enqueue", "ticket-a")
        record = (self.state / "ticket-a.json").read_bytes()
        self.assertNotIn(b"private", record)
        (self.work / "new-file").write_text("different untracked source\n")
        result = self.raw_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("candidate changed", result.stderr)
        self.assertFalse(self.calls.exists())
        self.assertEqual((self.state / "ticket-a.json").read_bytes(), record)
        (self.work / "new-file").write_text("private untracked source\n")
        self.mode.write_text("ready")
        self.assertEqual(self.raw_cli("wait", "ticket-a", "--slice", "0").returncode, 0)

    def test_changed_head_or_tracked_file_requires_explicit_repin(self):
        self.run_cli("enqueue", "ticket-a")
        (self.work / "source").write_text("changed\n")
        self.assertEqual(self.raw_cli("wait", "ticket-a", "--slice", "0").returncode, 2)
        self.pin_ticket("ticket-a")
        self.git("add", "source")
        self.git("commit", "-m", "changed")
        self.assertEqual(self.raw_cli("wait", "ticket-a", "--slice", "0").returncode, 2)
        self.assertFalse(self.calls.exists())

    def test_changed_remote_base_blocks_pinned_candidate(self):
        self.run_cli("enqueue", "ticket-a")
        # Advance only the fake remote, leaving the candidate untouched.
        commit = self.git("commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "base advance").strip()
        self.git("push", "origin", commit + ":refs/heads/main")
        result = self.raw_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("remote base changed", result.stderr)
        self.assertFalse(self.calls.exists())
        self.base = commit
        self.pin_ticket("ticket-a")
        self.assertEqual(self.raw_cli("wait", "ticket-a", "--slice", "0").returncode, 75)

    def test_source_change_during_remote_check_is_rejected(self):
        module = runpy.run_path(str(SCRIPT))
        verify = module["verify_proof"]
        original = module["git_output"]
        old_cwd = Path.cwd()
        os.chdir(self.work)
        self.addCleanup(os.chdir, old_cwd)
        proof = {"candidate": module["candidate_identity"](), "remote": "origin",
                 "base_ref": "refs/heads/main", "base_sha": self.base}

        def move_source(*arguments, **kwargs):
            output = original(*arguments, **kwargs)
            if arguments[0] == "ls-remote":
                (self.work / "source").write_text("changed during network lookup\n")
            return output

        with patch.dict(verify.__globals__, git_output=move_source):
            with self.assertRaisesRegex(module["Refused"], "candidate changed"):
                verify(proof)

    def test_legacy_acquired_session_remains_inspectable_after_source_changes(self):
        self.mode.write_text("ready")
        self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 0)
        path = self.state / "ticket-a.json"
        record = json.loads(path.read_text())
        del record["proof"]
        path.write_text(json.dumps(record))
        (self.work / "source").write_text("changed after allocation\n")
        self.assertEqual(self.raw_cli("wait", "ticket-a", "--slice", "0").returncode, 0)
        self.assertEqual(self.raw_cli("pin", "ticket-a", "--candidate", "0" * 64,
                                    "--base-sha", self.base, "--phase", "final").returncode, 2)
        self.mode.write_text("stopped")
        self.assertEqual(self.raw_cli("finish", "ticket-a", "--session-id", "ses_test").returncode, 0)

    def test_deadline_must_be_finite_and_bounded_before_ticket_creation(self):
        for value in ("nan", "inf", "-inf", "25", "1e308", "0", "0.5", "0.999"):
            with self.subTest(value=value):
                result = self.run_cli("enqueue", "ticket-a", "--deadline-hours", value)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertFalse((self.state / "ticket-a.json").exists())
                result = self.run_cli("wait", "ticket-a", "--deadline-hours", value, "--slice", "0")
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertFalse((self.state / "ticket-a.json").exists())

    def test_default_ticket_waits_at_least_one_hour_on_both_entry_points(self):
        for command in ("enqueue", "wait"):
            with self.subTest(command=command):
                ticket = f"default-{command}"
                args = ("--slice", "0") if command == "wait" else ()
                result = self.raw_cli(command, ticket, *args)
                self.assertEqual(result.returncode, 75 if command == "wait" else 0, result.stderr)
                record = json.loads((self.state / f"{ticket}.json").read_text())
                self.assertAlmostEqual(record["deadline"] - record["created"], 3600, delta=1)

    def test_hour_is_minimum_but_longer_explicit_deadline_is_allowed(self):
        for value in ("1", "8", "24"):
            with self.subTest(value=value):
                ticket = f"hour-{value}"
                result = self.raw_cli("enqueue", ticket, "--deadline-hours", value)
                self.assertEqual(result.returncode, 0, result.stderr)
                record = json.loads(result.stdout)
                self.assertAlmostEqual(record["deadline"] - record["created"], int(value) * 3600, delta=1)

    def test_wait_preserves_existing_deadline_including_a_legacy_short_ticket(self):
        self.assertEqual(self.raw_cli("enqueue", "ahead").returncode, 0)
        ticket = self.raw_cli("enqueue", "legacy-short", "--deadline-hours", "8")
        self.assertEqual(ticket.returncode, 0, ticket.stderr)
        path = self.state / "legacy-short.json"
        record = json.loads(path.read_text())
        record["deadline"] = record["created"] + 1800  # Pre-existing ticket, not a new request.
        path.write_text(json.dumps(record))
        result = self.raw_cli("wait", "legacy-short", "--slice", "0")
        self.assertEqual(result.returncode, 75, result.stderr)
        self.assertEqual(json.loads(path.read_text())["deadline"], record["deadline"])

    def test_legacy_invalid_queued_deadline_does_not_block_next_ticket(self):
        for index, invalid in enumerate((float("nan"), float("inf"), 1e308)):
            with self.subTest(invalid=invalid):
                self.state = self.root / f"legacy-state-{index}"
                self.assertEqual(self.run_cli("enqueue", "ticket-a").returncode, 0)
                path = self.state / "ticket-a.json"
                record = json.loads(path.read_text())
                record["deadline"] = invalid
                path.write_text(json.dumps(record))
                self.assertEqual(self.run_cli("enqueue", "ticket-b").returncode, 0)
                self.mode.write_text("ready")
                result = self.run_cli("wait", "ticket-b", "--slice", "0")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(path.read_text())["state"], "expired")
                self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 2)

    def test_legacy_invalid_starting_deadline_is_not_auto_cleared(self):
        self.assertEqual(self.run_cli("enqueue", "ticket-a").returncode, 0)
        path = self.state / "ticket-a.json"
        record = json.loads(path.read_text())
        record.update(state="starting", deadline=float("inf"))
        path.write_text(json.dumps(record))
        self.assertEqual(self.run_cli("enqueue", "ticket-b").returncode, 0)
        self.mode.write_text("ready")
        result = self.run_cli("wait", "ticket-b", "--slice", "0")
        self.assertEqual(result.returncode, 75, result.stderr)
        self.assertEqual(json.loads(path.read_text())["state"], "starting")
        self.assertFalse(self.calls.exists())


    def test_waiting_behind_an_active_start_is_not_an_error_or_second_rental(self):
        self.mode.write_text("slow-capacity")
        self.assertEqual(self.run_cli("enqueue", "ticket-a").returncode, 0)
        self.assertEqual(self.run_cli("enqueue", "ticket-b").returncode, 0)
        env = dict(os.environ, HOME=str(self.home), FAKE_CALLS=str(self.calls), FAKE_MODE=str(self.mode))
        first = subprocess.Popen([sys.executable, str(SCRIPT), "--state-dir", str(self.state),
                                  "--nomac", str(self.nomac), "wait", "ticket-a", "--slice", "0"],
                                 cwd=self.work, env=env, text=True, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE)
        try:
            for _ in range(100):
                if self.calls.exists() and "start" in self.calls.read_text():
                    break
                time.sleep(0.05)
            else:
                self.fail("first waiter never reached NoMac start")
            second = self.run_cli("wait", "ticket-b", "--slice", "0")
            self.assertEqual(second.returncode, 75, second.stderr)
            self.assertEqual(second.stdout, "")
            self.assertEqual(len([line for line in self.calls.read_text().splitlines()
                                  if line.startswith("start")]), 1)
        finally:
            first.communicate(timeout=10)
        self.assertEqual(first.returncode, 75)

    def test_acquired_session_is_durable_and_never_started_twice(self):
        self.mode.write_text("ready")
        result = self.run_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["session_id"], "ses_test")
        self.assertNotEqual(json.loads(result.stdout)["workspace"], str(self.work))
        again = self.run_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertEqual(len([x for x in self.calls.read_text().splitlines() if x.startswith("start")]), 1)
        self.assertEqual(self.run_cli("cancel", "ticket-a").returncode, 2)
        self.assertEqual(self.run_cli("finish", "ticket-a", "--session-id", "ses_other").returncode, 2)
        self.assertEqual(self.run_cli("finish", "ticket-a", "--session-id", "ses_test").returncode, 2)
        self.mode.write_text("stopped")
        self.assertEqual(self.run_cli("finish", "ticket-a", "--session-id", "ses_test").returncode, 0)
        self.assertEqual(json.loads(self.run_cli("status").stdout), [])

    def test_resumed_acquired_ticket_does_not_report_a_stopped_mac(self):
        self.mode.write_text("ready")
        self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 0)
        self.mode.write_text("stopped")
        result = self.run_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("is stopped", result.stderr)
        self.assertEqual(json.loads(self.run_cli("status").stdout)[0]["state"], "acquired")
        self.assertEqual(len([x for x in self.calls.read_text().splitlines() if x.startswith("start")]), 1)

    def test_nonready_session_holds_queue_and_becomes_ready_without_second_start(self):
        self.mode.write_text("queued")
        result = self.run_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(result.returncode, 75, result.stderr)
        self.assertEqual(json.loads(self.run_cli("status").stdout)[0]["session_id"], "ses_test")
        self.assertFalse(any(x.startswith("status") for x in self.calls.read_text().splitlines()))
        second = self.run_cli("wait", "ticket-b", "--slice", "0")
        self.assertEqual(second.returncode, 75, second.stderr)
        self.assertEqual(len([x for x in self.calls.read_text().splitlines() if x.startswith("start")]), 1)
        self.mode.write_text("ready")
        record = self.state / "ticket-a.json"
        data = json.loads(record.read_text())
        data["next_attempt"] = 0
        record.write_text(json.dumps(data))
        self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 0)
        self.assertEqual(len([x for x in self.calls.read_text().splitlines() if x.startswith("start")]), 1)

    def test_two_tickets_in_same_checkout_get_separate_nomac_workspaces(self):
        self.mode.write_text("ready")
        a = self.run_cli("wait", "ticket-a", "--slice", "0")
        b = self.run_cli("wait", "ticket-b", "--slice", "0")
        self.assertEqual(a.returncode, 0, a.stderr)
        self.assertEqual(b.returncode, 2, b.stderr)  # fake reused ID; reject it
        data = json.loads(self.run_cli("status").stdout)
        self.assertNotEqual(data[0]["workspace"], data[1]["workspace"])

    def test_recovery_cannot_adopt_another_tickets_session(self):
        self.mode.write_text("ready")
        self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 0)
        self.assertEqual(self.run_cli("wait", "ticket-b", "--slice", "0").returncode, 2)
        result = self.run_cli("recover", "ticket-b")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("already belongs to another ticket", result.stderr)
        records = {record["ticket"]: record for record in json.loads(self.run_cli("status").stdout)}
        self.assertEqual(records["ticket-a"]["state"], "acquired")
        self.assertEqual(records["ticket-b"]["state"], "starting")
        self.assertNotIn("session_id", records["ticket-b"])

    def test_unknown_start_blocks_queue_until_reconciled(self):
        self.mode.write_text("unknown")
        first = self.run_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(first.returncode, 2, first.stderr)
        self.assertEqual(json.loads(self.run_cli("status").stdout)[0]["state"], "starting")
        self.mode.write_text("ready")
        self.assertEqual(self.run_cli("wait", "ticket-b", "--slice", "0").returncode, 75)
        self.assertEqual(len([x for x in self.calls.read_text().splitlines() if x.startswith("start")]), 1)
        key = json.loads((self.state / "ticket-a.json").read_text())["request_key"]
        self.mode.write_text("no-session")
        self.assertEqual(self.run_cli("recover", "ticket-a").returncode, 0)
        self.mode.write_text("ready")
        self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 0)
        starts = [x for x in self.calls.read_text().splitlines() if x.startswith("start")]
        self.assertEqual(len(starts), 2)
        self.assertTrue(all(key in call for call in starts))

    def test_fifo_and_cancel(self):
        self.assertEqual(self.run_cli("enqueue", "first").returncode, 0)
        self.assertEqual(self.run_cli("enqueue", "second").returncode, 0)
        self.mode.write_text("ready")
        self.assertEqual(self.run_cli("wait", "second", "--slice", "0").returncode, 75)
        self.assertFalse(self.calls.exists())
        self.assertEqual(self.run_cli("cancel", "first").returncode, 0)
        result = self.run_cli("wait", "second", "--slice", "0")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_noncapacity_error_needs_recovery_before_next_rental(self):
        self.mode.write_text("failed")
        result = self.run_cli("wait", "ticket-a", "--slice", "0")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("insufficient_credit", result.stderr)
        self.assertEqual(len(self.calls.read_text().splitlines()), 1)
        self.assertEqual(json.loads(self.run_cli("status").stdout)[0]["state"], "starting")
        self.assertEqual(self.run_cli("wait", "other", "--slice", "0").returncode, 75)
        self.assertEqual(len([x for x in self.calls.read_text().splitlines() if x.startswith("start")]), 1)

    def test_postallocation_error_keeps_session_ownership(self):
        self.mode.write_text("poststart")
        self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 2)
        # A legacy uncertain allocation must still be recoverable without proof.
        path = self.state / "ticket-a.json"
        record = json.loads(path.read_text())
        del record["proof"]
        path.write_text(json.dumps(record))
        self.assertEqual(self.run_cli("cancel", "ticket-a").returncode, 2)
        self.assertEqual(self.run_cli("wait", "ticket-b", "--slice", "0").returncode, 75)
        self.mode.write_text("ready")
        self.assertEqual(self.run_cli("recover", "ticket-a").returncode, 0)
        self.assertEqual(json.loads(self.run_cli("wait", "ticket-a", "--slice", "0").stdout)["session_id"], "ses_test")
        self.assertEqual(len([x for x in self.calls.read_text().splitlines() if x.startswith("start")]), 1)

    def test_recover_rejects_unowned_attachment(self):
        self.mode.write_text("unknown")
        self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 2)
        record = json.loads((self.state / "ticket-a.json").read_text())
        name = hashlib.sha256(str(Path(record["workspace"]).resolve()).encode()).hexdigest()[:24]
        local = self.home / ".nomac" / "sessions" / name / "session.json"
        local.write_text(json.dumps({"mode": "hosted", "requestKey": "other", "id": "ses_test"}))
        self.mode.write_text("ready")
        result = self.run_cli("recover", "ticket-a")
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads((self.state / "ticket-a.json").read_text())["state"], "starting")

    def test_cancel_requires_original_checkout(self):
        self.assertEqual(self.run_cli("enqueue", "ticket-a").returncode, 0)
        other = self.root / "other"
        other.mkdir()
        env = dict(os.environ, HOME=str(self.home), FAKE_CALLS=str(self.calls), FAKE_MODE=str(self.mode))
        result = subprocess.run([sys.executable, str(SCRIPT), "--state-dir", str(self.state),
                                 "--nomac", str(self.nomac), "cancel", "ticket-a"],
                                cwd=other, env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertTrue((self.state / "ticket-a.json").exists())

    def test_finish_requires_original_checkout(self):
        self.mode.write_text("ready")
        self.assertEqual(self.run_cli("wait", "ticket-a", "--slice", "0").returncode, 0)
        self.mode.write_text("stopped")
        other = self.root / "other"
        other.mkdir()
        env = dict(os.environ, HOME=str(self.home), FAKE_CALLS=str(self.calls), FAKE_MODE=str(self.mode))
        result = subprocess.run([sys.executable, str(SCRIPT), "--state-dir", str(self.state),
                                 "--nomac", str(self.nomac), "finish", "ticket-a",
                                 "--session-id", "ses_test"], cwd=other, env=env,
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertTrue((self.state / "ticket-a.json").exists())


if __name__ == "__main__":
    unittest.main()
