#!/usr/bin/env python3
"""SSH-form transport for disposable containers; it never invokes real SSH."""
import fcntl
import json
import os
import re
import shlex
import subprocess
import sys
from pathlib import Path


PINNED = json.loads(Path(__file__).with_name("fixtures.json").read_text())

def main():
    args = sys.argv[1:]
    if "-G" in args:
        return 1
    if len(args) < 2:
        raise ValueError("Missing transport arguments")
    host = args[-2]
    command = shlex.split(args[-1])
    if host not in ("git@github.com", "github.com", "git@polka-dot-cat.git.beanstalkapp.com"):
        raise ValueError("Unexpected fixture host")
    if len(command) not in (2, 3):
        raise ValueError("Unexpected transport command")
    operation, name = command[:2]
    name = name.removesuffix(".git")
    if not re.fullmatch(r"[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+", name) or ".." in name:
        raise ValueError("Invalid fixture name")
    custom = os.environ.get("FIXTURE_CUSTOM_URLS") == "1"
    if custom and name != "polka-dot-cat/felidae":
        raise ValueError("Unexpected custom URL fixture")
    if host.endswith("beanstalkapp.com") and not (custom and name == "polka-dot-cat/felidae"):
        raise ValueError("Unexpected Beanstalk fixture")
    if os.environ.get("FIXTURE_OFFLINE") == "1":
        if not int(Path("/sys/class/net/eth0/flags").read_text(), 16) & 1:
            raise ValueError("Network interface is down")
    if operation == "git-lfs-authenticate" and command[2:] == ["download"]:
        if custom:
            raise ValueError("Custom URL fixtures have no network transport")
        print(json.dumps({"href": "https://github.com/" + name + ".git/info/lfs"}))
        return 0
    if operation not in ("git-upload-pack", "git-receive-pack") or len(command) != 2:
        raise ValueError("Unexpected Git operation")
    if operation == "git-receive-pack" and not (os.environ.get("FIXTURE_WRITES") == "1" and name in PINNED):
        raise ValueError("Writes are limited to private pinned fixtures")
    path = Path("/fixtures") / (name + ".git")
    path.parent.mkdir(parents=True, exist_ok=True)
    # Several parallel projects can request the same upstream for the first time.
    with path.with_suffix(".lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if not path.is_dir():
            if custom:
                raise ValueError("Missing private fixture")
            # Reads of additional upstream repositories use public HTTPS only.
            subprocess.run(["git", "clone", "--bare", "https://github.com/" + name + ".git", str(path)],
                           check=True, stdout=sys.stderr)
            subprocess.run(["git", "-C", str(path), "remote", "remove", "origin"], check=True)
    os.execvp(operation, [operation, str(path)])


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
