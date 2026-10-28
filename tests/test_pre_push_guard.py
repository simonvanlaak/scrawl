"""Exercise the actual pre-push hook in disposable Git repositories."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


HOOK = Path(__file__).resolve().parents[1] / ".githooks" / "pre-push"


def run(*args, cwd, env=None, input=None):
    return subprocess.run(args, cwd=cwd, env=env, input=input, text=True,
                          capture_output=True, check=False)


class PrePushGuardTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name) / "repo"
        self.repo.mkdir()
        run("git", "init", "-q", str(self.repo), cwd=self.repo)
        run("git", "config", "user.name", "Test", cwd=self.repo)
        run("git", "config", "user.email", "test@example.invalid", cwd=self.repo)
        (self.repo / "file").write_text("original\n")
        run("git", "add", "file", cwd=self.repo)
        run("git", "commit", "-qm", "original", cwd=self.repo)
        self.head = run("git", "rev-parse", "HEAD", cwd=self.repo).stdout.strip()
        self.bin = Path(self.tmp.name) / "bin"
        self.bin.mkdir()
        self.log = Path(self.tmp.name) / "commands"
        taskpolicy = self.bin / "taskpolicy"
        taskpolicy.write_text("#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$COMMAND_LOG\"\nexit \"${MOCK_STATUS:-0}\"\n")
        taskpolicy.chmod(0o755)
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}", COMMAND_LOG=str(self.log))

    def push(self, oid=None, local="refs/heads/feature", remote="refs/heads/feature", env=None):
        line = f"{local} {oid or self.head} {remote} {'0' * 40}\n"
        return run("sh", str(HOOK), "origin", "example.invalid:repo", cwd=self.repo,
                   env=env or self.env, input=line)

    def test_clean_head_requires_build_and_tests(self):
        result = self.push()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.log.read_text().splitlines(), [
            "-c utility cargo build -j 3 --offline",
            "-c utility cargo test -j 3 --offline -- --test-threads=2",
        ])

    def test_failed_verification_rejects_push(self):
        result = self.push(env=dict(self.env, MOCK_STATUS="1"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("failed", result.stderr)

    def test_tree_changed_during_verification_rejects_push(self):
        taskpolicy = self.bin / "taskpolicy"
        taskpolicy.write_text("#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$COMMAND_LOG\"\n"
                              "printf 'later\\n' > file\n")
        result = self.push()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("changed during verification", result.stderr)

    def test_dirty_tree_rejects_before_verification(self):
        (self.repo / "file").write_text("changed\n")
        result = self.push()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.log.exists())

    def test_hidden_index_flags_reject_before_verification(self):
        for flag in ("--assume-unchanged", "--skip-worktree"):
            with self.subTest(flag=flag):
                (self.repo / "file").write_text("different bytes\n")
                run("git", "update-index", flag, "file", cwd=self.repo)
                result = self.push()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("hidden index flags", result.stderr)
                self.assertFalse(self.log.exists())
                run("git", "update-index", "--no-assume-unchanged", "file", cwd=self.repo)
                run("git", "update-index", "--no-skip-worktree", "file", cwd=self.repo)
                (self.repo / "file").write_text("original\n")

    def test_other_revision_rejects_before_verification(self):
        run("git", "commit", "--allow-empty", "-qm", "newer", cwd=self.repo)
        result = self.push()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.log.exists())

    def test_multiple_refs_cannot_hide_an_old_revision(self):
        newer = run("git", "commit", "--allow-empty", "-qm", "newer", cwd=self.repo)
        self.assertEqual(newer.returncode, 0)
        tip = run("git", "rev-parse", "HEAD", cwd=self.repo).stdout.strip()
        lines = (f"refs/heads/current {tip} refs/heads/current {'0' * 40}\n"
                 f"refs/heads/old {self.head} refs/heads/old {'0' * 40}\n")
        result = run("sh", str(HOOK), "origin", "example.invalid:repo", cwd=self.repo,
                     env=self.env, input=lines)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.log.exists())

    def test_ref_deletion_does_not_need_build(self):
        result = self.push(oid="0" * 40, local="(delete)")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(self.log.exists())

    def test_git_push_runs_installed_hook_and_blocks_failed_build(self):
        hooks = Path(self.tmp.name) / "hooks"
        hooks.mkdir()
        shutil.copy2(HOOK, hooks / "pre-push")
        (hooks / "pre-push").chmod(0o755)
        remote = Path(self.tmp.name) / "remote.git"
        run("git", "init", "--bare", "-q", str(remote), cwd=self.repo)
        run("git", "config", "core.hooksPath", str(hooks), cwd=self.repo)
        run("git", "remote", "add", "origin", str(remote), cwd=self.repo)

        failed = run("git", "push", "origin", "HEAD:refs/heads/test", cwd=self.repo,
                     env=dict(self.env, MOCK_STATUS="1"))
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("build failed", failed.stderr)
        self.assertNotEqual(run("git", "show-ref", "--verify", "refs/heads/test",
                                cwd=remote).returncode, 0)

        passed = run("git", "push", "origin", "HEAD:refs/heads/test", cwd=self.repo,
                     env=self.env)
        self.assertEqual(passed.returncode, 0, passed.stderr)
        self.assertEqual(run("git", "rev-parse", "refs/heads/test", cwd=remote).stdout.strip(),
                         self.head)

        (self.repo / "file").write_text("hidden change\n")
        run("git", "update-index", "--assume-unchanged", "file", cwd=self.repo)
        hidden = run("git", "push", "origin", "HEAD:refs/heads/hidden", cwd=self.repo,
                     env=self.env)
        self.assertNotEqual(hidden.returncode, 0)
        self.assertIn("hidden index flags", hidden.stderr)
        self.assertNotEqual(run("git", "show-ref", "--verify", "refs/heads/hidden",
                                cwd=remote).returncode, 0)


if __name__ == "__main__":
    unittest.main()
