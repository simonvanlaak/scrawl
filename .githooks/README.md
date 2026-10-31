# Git hooks

Enable the repository hooks after cloning:

```sh
git config --local core.hooksPath .githooks
```

This replaces any configured hooks path for this repository and applies to its
linked worktrees. Inspect an existing custom hook setup before replacing it.
Git does not install hooks automatically when cloning or pulling.

`pre-commit` requires Python 3 and checks added, modified, copied or renamed
regular text files in the **index**, not unstaged working-tree contents. The
maximum is **2,000 physical lines**, including comments and blank lines; an
unterminated final line counts. Files at exactly 2,000 lines pass. There are no
extension exemptions, including for documentation, generated text or lockfiles.
Files containing a NUL byte in their first 8,000 bytes are treated as binary;
symlinks and submodules are skipped. Unchanged oversized files do not block
unrelated commits, but editing or renaming one requires bringing it within the
limit. Deletions are allowed. The hook never formats, stages or rewrites files.

`pre-push` retains the existing exact-HEAD build and test verification.
Local hooks are a developer guard, not server-side enforcement: Git can bypass
them with `--no-verify` or a different hooks path.

Test the hooks without touching real repository state:

```sh
python3 -m unittest discover -s tests -p 'test_pre_commit_line_limit.py'
python3 -m unittest discover -s tests -p 'test_pre_push_guard.py'
```
