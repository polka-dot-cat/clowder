"""Build private bare fixtures from public HTTPS, with recorded heads and tags."""
import json
import os
import subprocess
from pathlib import Path


def git(path, *args):
    return subprocess.check_output(["git", "-C", str(path), *args], text=True)


if os.environ.get("CLOWDER_ISOLATED_TESTS") != "1":
    raise SystemExit("Use script/test-isolated build")

for name, fixture in json.loads(Path(__file__).with_name("fixtures.json").read_text()).items():
    path = Path("/fixtures") / (name + ".git")
    path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "clone", "--bare", "https://github.com/" + name + ".git", str(path)], check=True)
    for ref in git(path, "for-each-ref", "--format=%(refname)", "refs/heads", "refs/tags").splitlines():
        git(path, "update-ref", "-d", ref)
    for ref, sha in fixture["refs"].items():
        # Fail if a recorded object is no longer available; never silently move it.
        git(path, "cat-file", "-e", sha)
        git(path, "update-ref", ref, sha)
    git(path, "symbolic-ref", "HEAD", fixture["head"])
    # No writable fixture retains a GitHub push destination.
    git(path, "remote", "remove", "origin")
