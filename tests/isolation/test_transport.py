"""Check the fixture transport boundary without network access or Git mutations."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("fixture_ssh", Path(__file__).with_name("ssh.py"))
ssh = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ssh)


class Executed(Exception):
    pass


class TransportTests(unittest.TestCase):
    def invoke(self, host, command, environment=None):
        with patch("sys.argv", ["ssh.py", host, command]), patch.dict(os.environ, environment or {}, clear=True):
            return ssh.main()

    @contextlib.contextmanager
    def private_fixture(self):
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.object(Path, "is_dir", return_value=True))
            stack.enter_context(patch.object(Path, "mkdir"))
            stack.enter_context(patch.object(Path, "open", unittest.mock.mock_open()))
            stack.enter_context(patch.object(ssh.fcntl, "flock"))
            execute = stack.enter_context(patch.object(ssh.os, "execvp", side_effect=Executed))
            yield execute

    def test_rejects_unexpected_hosts_paths_and_commands(self):
        for host, command in [
            ("other.example", "git-upload-pack polka-dot-cat/felidae.git"),
            ("git@github.com", "git-upload-pack ../../etc/passwd"),
            ("git@github.com", "git-upload-pack /polka-dot-cat/felidae.git"),
            ("git@github.com", "git-upload-pack polka-dot-cat/felidae.git; touch /escaped"),
            ("git@github.com", "git-lfs-authenticate polka-dot-cat/felidae.git upload"),
            ("git@polka-dot-cat.git.beanstalkapp.com", "git-upload-pack polka-dot-cat/felidae.git"),
        ]:
            with self.subTest(host=host, command=command), self.assertRaises(ValueError):
                self.invoke(host, command)

    def test_receive_pack_requires_private_allowlist_and_explicit_mode(self):
        for environment, name in [({}, "polka-dot-cat/felidae"), ({"FIXTURE_WRITES": "1"}, "someone/else")]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.invoke("git@github.com", "git-receive-pack " + name, environment)
        with self.private_fixture() as execute:
            with self.assertRaises(Executed):
                self.invoke("git@github.com", "git-receive-pack polka-dot-cat/mu.git", {"FIXTURE_WRITES": "1"})
            execute.assert_called_once_with("git-receive-pack", ["git-receive-pack", "/fixtures/polka-dot-cat/mu.git"])

    def test_custom_urls_use_only_the_private_felidae_copy(self):
        with self.private_fixture() as execute:
            with self.assertRaises(Executed):
                self.invoke("git@polka-dot-cat.git.beanstalkapp.com", "git-upload-pack polka-dot-cat/felidae.git", {"FIXTURE_CUSTOM_URLS": "1"})
            execute.assert_called_once_with("git-upload-pack", ["git-upload-pack", "/fixtures/polka-dot-cat/felidae.git"])
        with self.assertRaises(ValueError):
            self.invoke("git@github.com", "git-upload-pack polka-dot-cat/mu.git", {"FIXTURE_CUSTOM_URLS": "1"})

    def test_offline_interface_blocks_private_reads_and_writes(self):
        with patch.object(Path, "read_text", return_value="0x1002"):
            for operation in ["git-upload-pack", "git-receive-pack"]:
                with self.subTest(operation=operation), self.assertRaisesRegex(ValueError, "interface is down"):
                    self.invoke("git@github.com", operation + " polka-dot-cat/mu.git", {"FIXTURE_OFFLINE": "1", "FIXTURE_WRITES": "1"})

    def test_lfs_authorization_is_download_only_and_has_no_credentials(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.assertEqual(self.invoke("git@github.com", "git-lfs-authenticate polka-dot-cat/felidae.git download"), 0)
        self.assertEqual(json.loads(output.getvalue()), {"href": "https://github.com/polka-dot-cat/felidae.git/info/lfs"})


if __name__ == "__main__":
    unittest.main()
