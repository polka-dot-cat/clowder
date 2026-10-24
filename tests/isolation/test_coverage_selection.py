"""A CI shell startup file must not replace the instrumented CLI selection."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class CoverageSelectionTests(unittest.TestCase):
    def test_bash_launcher_keeps_the_instrumented_executable_and_path(self):
        with tempfile.TemporaryDirectory(prefix="clowder-coverage-selection-") as directory:
            root = Path(directory)
            (root / "script").mkdir()
            script = root / "script/coverage-rust"
            shutil.copy2(Path(__file__).resolve().parents[2] / "script/coverage-rust", script)
            tools = root / "tools"
            tools.mkdir()
            ordinary = root / "ordinary"
            ordinary.mkdir()
            instrumented = root / "target/rust-coverage/debug"
            instrumented.mkdir(parents=True)

            def executable(path, text):
                path.write_text(text)
                path.chmod(0o755)

            executable(tools / "cargo", '#!/bin/sh\n[ -z "${BASH_ENV+x}" ]\n')
            executable(tools / "rustc", "#!/bin/sh\necho 'rustc test stub'\n")
            executable(tools / "cargo-llvm-cov", """#!/bin/sh
case "$2" in
  --version) echo 'cargo-llvm-cov 0.9.1' ;;
  show-env) echo 'export LLVM_PROFILE_FILE=test.profraw' ;;
  report) echo 'test report' ;;
esac
""")
            for path in [ordinary / "clowder", instrumented / "clowder"]:
                executable(path, "#!/bin/sh\nexit 0\n")
            startup = root / "ci-startup.sh"
            startup.write_text('export CLOWDER_EXECUTABLE="$ORDINARY/clowder"\nexport PATH="$ORDINARY:$PATH"\n')
            # pyenv's Python launcher is a Bash script. Bash reads BASH_ENV
            # again before exec, so ordinary CI exports can replace later ones.
            launcher = root / "python-shim"
            executable(launcher, """#!/usr/bin/env bash
set -eu
[[ "$CLOWDER_EXECUTABLE" == "$EXPECTED/clowder" ]]
[[ "$(command -v clowder)" == "$EXPECTED/clowder" ]]
printf 'instrumented selection retained\\n'
""")
            env = os.environ.copy()
            env.update(BASH_ENV=str(startup), ORDINARY=str(ordinary), EXPECTED=str(instrumented),
                       PATH=str(tools) + os.pathsep + env.get("PATH", ""),
                       RUSTC=str(tools / "rustc"), CARGO_LLVM_COV_BIN=str(tools / "cargo-llvm-cov"))
            result = subprocess.run([str(script), "--", str(launcher)], env=env, text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertIn("instrumented selection retained", result.stdout)
            self.assertIn("Exit status: 0", (root / "build/coverage/rust/run.txt").read_text())


if __name__ == "__main__":
    unittest.main()
