#!/usr/bin/env python3
"""Fair, bounded foreground wait for an owned NoMac session.

Exit 75 means still queued: call wait again in the SAME T3 turn. Exit 0
prints JSON with the acquired session ID. Never attaches to another Mac.
"""
import argparse
from contextlib import contextmanager

import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import uuid

DEFAULT_STATE = Path.home() / ".local/state/nomac-wait"
TICKET = re.compile(r"[a-zA-Z0-9][a-zA-Z0-9_-]{0,79}\Z")
CAPACITY = "session_limit_reached"


class Refused(Exception):
    pass

class Busy(Refused):
    pass


def timestamp():
    return time.time()


def prepare(directory):
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    if directory.is_symlink() or not directory.is_dir() or directory.stat().st_uid != os.geteuid():
        raise Refused("state directory must be a real, user-owned directory")
    directory.chmod(0o700)


@contextmanager
def locked(directory):
    prepare(directory)
    fd = os.open(directory / ".lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        limit = time.monotonic() + 5
        while True:
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= limit:
                    raise Busy("queue busy; retry the same ticket")
                time.sleep(0.1)
        yield
    finally:
        fcntl.flock(fd, fcntl.LOCK_UN)
        os.close(fd)


def path_for(directory, ticket):
    if not TICKET.fullmatch(ticket):
        raise Refused("ticket must be 1–80 letters, digits, underscores or hyphens")
    return directory / (ticket + ".json")


def load(path):
    if path.is_symlink():
        raise Refused(f"symlink ticket refused: {path}")
    if not path.exists():
        return None
    if not path.is_file() or path.stat().st_uid != os.geteuid():
        raise Refused(f"foreign ticket refused: {path}")
    return json.loads(path.read_text())


def save(path, record):
    tmp = path.with_name(path.name + f".{os.getpid()}.tmp")
    fd = os.open(tmp, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "w") as stream:
            json.dump(record, stream, sort_keys=True)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(tmp, path)
    finally:
        tmp.unlink(missing_ok=True)


def records(directory):
    result = []
    for path in directory.glob("*.json"):
        record = load(path)
        if record and (record.get("state") in ("starting", "provisioning") or
                       (record.get("state") == "queued" and record["deadline"] > timestamp())):
            result.append(record)
    return sorted(result, key=lambda record: (record["created"], record["ticket"]))


def enqueue(directory, ticket, deadline):
    path = path_for(directory, ticket)
    workdir = str(Path.cwd().resolve())
    with locked(directory):
        record = load(path)
        if record:
            if record["workdir"] != workdir:
                raise Refused("ticket belongs to another working directory")
            return record
        workspace = directory / "workspaces" / ticket
        workspace.mkdir(mode=0o700, parents=True, exist_ok=False)
        record = {"ticket": ticket, "workdir": workdir, "created": timestamp(),
                  "workspace": str(workspace), "deadline": deadline,
                  "request_key": "nomac-wait-" + uuid.uuid4().hex,
                  "next_attempt": 0, "state": "queued"}
        save(path, record)
        return record


def start(nomac, workdir, duration, idle, cost, request_key):
    command = [*nomac, "start", "--parallel", "--drive", "none", "--max-duration",
               str(duration), "--idle-timeout", str(idle), "--max-cost", str(cost),
               "--wait", "120", "--request-key", request_key, "--json"]
    try:
        result = subprocess.run(command, cwd=workdir, text=True, capture_output=True, timeout=165)
    except (subprocess.TimeoutExpired, OSError) as exc:
        raise Refused(f"start outcome unknown; inspect NoMac before retrying: {exc}") from exc
    try:
        response = json.loads(result.stdout if result.returncode == 0 else result.stderr)
    except json.JSONDecodeError as exc:
        raise Refused(f"start returned non-JSON (exit {result.returncode}); inspect NoMac before retrying") from exc
    if result.returncode:
        code = response.get("error", {}).get("code")
        if code == CAPACITY:
            return None
        # An error after allocation (for example SSH setup) may leave a live Mac.
        raise Refused(f"NoMac start failed: {code or result.returncode}; inspect ticket workspace")
    session = response.get("session", {})
    session_id = session.get("id")
    if not isinstance(session_id, str) or not session_id.startswith("ses_"):
        raise Refused("start succeeded without an identifiable session; inspect NoMac")
    return session_id, session.get("state")


def existing_session(nomac, workspace, session_id):
    result = subprocess.run([*nomac, "status", "--json"], cwd=workspace,
                            text=True, capture_output=True, timeout=30)
    try:
        session = json.loads(result.stdout).get("session") or {}
    except json.JSONDecodeError as exc:
        raise Refused("could not read NoMac session status") from exc
    if result.returncode or session.get("id") != session_id:
        raise Refused("ticket workspace does not report its owned session")
    return session.get("state")


def local_nomac_state(workspace):
    # NoMac keys local session metadata by the real project path. It writes
    # requestKey before dispatch, so an unknown response can use the same key.
    name = hashlib.sha256(str(Path(workspace).resolve()).encode()).hexdigest()[:24]
    path = Path.home() / ".nomac" / "sessions" / name / "session.json"
    if path.is_symlink():
        raise Refused("NoMac session metadata is a symlink")
    return json.loads(path.read_text()) if path.exists() else None


def finish(directory, ticket, session_id, nomac):
    path = path_for(directory, ticket)
    with locked(directory):
        record = load(path)
        if not record or record.get("state") not in ("acquired", "provisioning") or record.get("session_id") != session_id:
            raise Refused("ticket does not own that acquired session")
        if record["workdir"] != str(Path.cwd().resolve()):
            raise Refused("ticket belongs to another working directory")
        result = subprocess.run([*nomac, "status", "--json"], cwd=record["workspace"],
                                text=True, capture_output=True, timeout=30)
        try:
            status = json.loads(result.stdout)
        except json.JSONDecodeError as exc:
            raise Refused("cannot verify stopped session from NoMac status") from exc
        session = status.get("session") or {}
        if result.returncode or session.get("id") != session_id or session.get("state") != "stopped" or not session.get("cleanup_confirmed"):
            raise Refused("NoMac has not confirmed this session stopped and cleaned up")
        path.unlink()


def wait(args):
    directory = Path(args.state_dir).expanduser()
    path = path_for(directory, args.ticket)
    deadline = timestamp() + args.deadline_hours * 3600
    enqueue(directory, args.ticket, deadline)
    end = timestamp() + args.slice
    while True:
        with locked(directory):
            record = load(path)
            if record is None:
                raise Refused("ticket was cancelled")
            if record.get("limits") and record["limits"] != [args.max_duration, args.idle_timeout, args.max_cost]:
                raise Refused("ticket was started with different session caps")
            now = timestamp()
            if record["state"] == "provisioning" and now >= record["next_attempt"]:
                state = existing_session(args.nomac, record["workspace"], record["session_id"])
                if state == "ready":
                    record["state"] = "acquired"
                    save(path, record)
                elif state not in ("queued", "provisioning"):
                    raise Refused(f"owned NoMac session became {state}; inspect it")
                else:
                    record["next_attempt"] = timestamp() + 10
                    save(path, record)
            if record["state"] == "acquired":
                state = existing_session(args.nomac, record["workspace"], record["session_id"])
                if state != "ready":
                    raise Refused(f"owned NoMac session is {state}; inspect it before continuing")
                print(json.dumps({"ticket": args.ticket, "session_id": record["session_id"],
                                  "workspace": record["workspace"]}))
                return 0
            if record["state"] not in ("queued", "provisioning"):
                raise Refused(f"ticket requires inspection: {record['state']}")
            now = timestamp()
            if now >= record["deadline"]:
                if record["state"] == "provisioning":
                    raise Refused("deadline expired with an owned session; inspect and stop it")
                record["state"] = "expired"
                save(path, record)
                raise Refused("ticket deadline expired")
            queue = records(directory)
            # At most 400s of waiting + a 165s start + 5s lock < 590s.
            if record["state"] == "queued" and queue and queue[0]["ticket"] == args.ticket and now >= record["next_attempt"]:
                # Save before calling the provider: after a crash, don't automatically
                # repeat an allocation with an unknown outcome.
                record["state"] = "starting"
                record["limits"] = [args.max_duration, args.idle_timeout, args.max_cost]
                save(path, record)
                try:
                    allocated = start(args.nomac, record["workspace"], args.max_duration,
                                      args.idle_timeout, args.max_cost, record["request_key"])
                except Refused:
                    # Leave 'starting' for manual recovery, not a second rental.
                    raise
                if allocated:
                    session_id, state = allocated
                    if any(other and other.get("session_id") == session_id for other in
                           (load(p) for p in directory.glob("*.json") if p != path)):
                        raise Refused(f"session {session_id} already belongs to another ticket")
                    record["session_id"] = session_id
                    record["state"] = "acquired" if state == "ready" else "provisioning"
                    if record["state"] == "provisioning":
                        record["next_attempt"] = timestamp() + 10
                    save(path, record)
                    if state == "ready":
                        print(json.dumps({"ticket": args.ticket, "session_id": session_id,
                                          "workspace": record["workspace"]}))
                        return 0
                    if state not in ("queued", "provisioning"):
                        raise Refused(f"session {session_id} is {state}; inspect ticket workspace")
                    # Check this session's status on the next iteration, not a second start.
                    continue
                record["state"] = "queued"
                # A definitive capacity rejection created no session. The next
                # attempt is a new allocation, not a retry of that rejection.
                record["request_key"] = "nomac-wait-" + uuid.uuid4().hex
                record["next_attempt"] = timestamp() + args.poll
                save(path, record)
            next_attempt = record["next_attempt"]
        if timestamp() >= end:
            return 75
        time.sleep(min(2, max(0.01, end - timestamp()), max(0.01, next_attempt - timestamp())))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--state-dir", default=str(DEFAULT_STATE))
    parser.add_argument("--nomac", nargs=1, default=["npx", "-y", "@nomac/cli"],
                        help="single NoMac CLI executable (for testing)")
    sub = parser.add_subparsers(dest="command", required=True)
    enqueue_parser = sub.add_parser("enqueue")
    enqueue_parser.add_argument("ticket")
    enqueue_parser.add_argument("--deadline-hours", type=float, default=24)
    waiter = sub.add_parser("wait")
    waiter.add_argument("ticket")
    waiter.add_argument("--deadline-hours", type=float, default=24)
    waiter.add_argument("--poll", type=float, default=300)
    waiter.add_argument("--slice", type=float, default=400)
    waiter.add_argument("--max-duration", type=int, default=1800)
    waiter.add_argument("--idle-timeout", type=int, default=300)
    waiter.add_argument("--max-cost", type=float, default=0.50)
    sub.add_parser("status")
    cancel = sub.add_parser("cancel")
    cancel.add_argument("ticket")
    recover = sub.add_parser("recover", help="reconcile an uncertain start with its original idempotency key")
    recover.add_argument("ticket")
    finished = sub.add_parser("finish")
    finished.add_argument("ticket")
    finished.add_argument("--session-id", required=True)
    args = parser.parse_args()
    try:
        if args.command in ("wait", "enqueue") and args.deadline_hours <= 0:
            raise Refused("deadline must be positive")
        if args.command == "wait":
            if not 0 <= args.slice <= 400 or args.poll <= 0:
                raise Refused("slice must be 0–400 seconds and poll must be positive")
            if not 0 < args.idle_timeout < args.max_duration <= 1800 or not 0 < args.max_cost <= 0.50:
                raise Refused("explicit session caps exceeded (30m, $0.50)")
            return wait(args)
        directory = Path(args.state_dir).expanduser()
        if args.command == "enqueue":
            print(json.dumps(enqueue(directory, args.ticket, timestamp() + args.deadline_hours * 3600)))
        elif args.command == "status":
            with locked(directory):
                print(json.dumps([load(p) for p in sorted(directory.glob("*.json"))]))
        elif args.command == "cancel":
            with locked(directory):
                path = path_for(directory, args.ticket)
                record = load(path)
                if record and record["workdir"] != str(Path.cwd().resolve()):
                    raise Refused("ticket belongs to another working directory")
                if record and record["state"] in ("acquired", "starting", "provisioning"):
                    raise Refused("session may be live; inspect and stop your own Mac before clearing ticket")
                path.unlink(missing_ok=True)
        elif args.command == "recover":
            with locked(directory):
                path = path_for(directory, args.ticket)
                record = load(path)
                if not record or record["state"] != "starting" or not record.get("request_key"):
                    raise Refused("only an uncertain start with its original request key can be recovered")
                if record["workdir"] != str(Path.cwd().resolve()):
                    raise Refused("ticket belongs to another working directory")
                local = local_nomac_state(record["workspace"])
                if local and (local.get("mode") != "hosted" or
                              local.get("requestKey") != record["request_key"]):
                    raise Refused("NoMac workspace belongs to a different allocation")
                result = subprocess.run([*args.nomac, "status", "--json"],
                                        cwd=record["workspace"], text=True,
                                        capture_output=True, timeout=30)
                if result.returncode == 0:
                    try:
                        session = json.loads(result.stdout).get("session") or {}
                    except json.JSONDecodeError as exc:
                        raise Refused("NoMac status is not JSON; keep ticket for inspection") from exc
                    session_id = session.get("id")
                    if session_id:
                        if not local or local.get("id") != session_id:
                            raise Refused("NoMac status session has no matching local allocation")
                        if any(other and other.get("session_id") == session_id for other in
                               (load(p) for p in directory.glob("*.json") if p != path)):
                            raise Refused(f"session {session_id} already belongs to another ticket")
                        record["session_id"] = session_id
                        state = session.get("state")
                        record["state"] = "acquired" if state == "ready" else "provisioning"
                        record["next_attempt"] = 0
                        save(path, record)
                        if state not in ("ready", "queued", "provisioning"):
                            raise Refused(f"owned session is {state}; inspect and stop it")
                        return 0
                if local and (local.get("id") or local.get("requestKey") != record["request_key"]):
                    raise Refused("NoMac has a session or different request key; inspect before recovery")
                # The CLI persisted its request key before dispatch. The next
                # start from this exact folder uses that same API idempotency key.
                record["state"] = "queued"
                record["next_attempt"] = 0
                save(path, record)
        elif args.command == "finish":
            finish(directory, args.ticket, args.session_id, args.nomac)
        return 0
    except Busy as exc:
        # Another worker may hold the lock through a bounded NoMac start.
        # An existing, owned ticket is still waiting, not a failed allocation.
        if args.command == "wait":
            record = load(path_for(Path(args.state_dir).expanduser(), args.ticket))
            if record and record["workdir"] == str(Path.cwd().resolve()):
                return 75
        print(f"nomac-wait: {exc}", file=sys.stderr)
        return 2
    except (Refused, ValueError, KeyError, OSError) as exc:
        print(f"nomac-wait: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
