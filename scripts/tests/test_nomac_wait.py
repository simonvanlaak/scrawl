"""Queue acquisition without renting a Mac; the fake CLI implements start/status."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "nomac-wait.py"


class NoMacWaitTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.work = self.root / "work"
        self.work.mkdir()
        self.state = self.root / "state"
        self.home = self.root / "home"
        self.home.mkdir()
        self.mode = self.root / "mode"
        self.mode.write_text("capacity")
        self.calls = self.root / "calls"
        self.nomac = self.root / "nomac"
        self.nomac.write_text("#!/usr/bin/env python3\n"
            "import hashlib, json, os, pathlib, sys\n"
            "pathlib.Path(os.environ['FAKE_CALLS']).open('a').write(' '.join(sys.argv[1:])+'\\n')\n"
            "mode=pathlib.Path(os.environ['FAKE_MODE']).read_text()\n"
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

    def run_cli(self, *args):
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
