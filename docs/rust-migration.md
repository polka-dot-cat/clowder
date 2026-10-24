# Rust Migration

Port clowder cli from python to rust.

Goals:

- Better performance
- Better portability (single binary)
- Better multithreading (especially for parallel clowder command options)

Keep all current BDD Gherkin files. So also keep python test harness and runner if that's the easiest route. All current tests are functional tests and call the cli interface so this should be a clean layer of separation. I only care about porting the actual cli to rust.

Before porting, feel free to create more tests that capture and preserve command output formatting. Tests will need to ignore parts that predictably change between test runs, but much should be consistent between runs. Currently many tests just confirm that commands exit successfully (i.e. help commands), but don't check the actual output.

Commit and push to `rust-migration` branch as you make progress. Check CI validation status on PR.

While working on this migration, check ChatGPT usage periodically. At 10% remaining or below, use one reset without asking. Keep the second reset in reserve.

## Build and validate the native CLI

The migration is in progress on `rust-migration` and draft PR #642. The transfer
cleanup branch is merged into this branch. The Python implementation remains as
a behavior reference and supplies utilities used by the unchanged BDD harness.
The native executable runs its own commands and does not invoke Python.

With Rust 1.99 or later and Git installed:

```sh
cargo build --locked
./target/debug/clowder --help
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

Use the existing Python test environment to run the BDD suite against Rust:

```sh
export CLOWDER_EXECUTABLE="$PWD/target/debug/clowder"
export PYTHONPATH="$PWD"
pipenv run python -m pytest tests/functional -q \
  -m 'not ssh and not offline and not write and not lfs'
```

Unset `CLOWDER_EXECUTABLE` to compare with Python. The Rust build itself needs
no Python installation. Git handles credentials, transports, and repository
configuration. Git LFS is needed only for manifests that enable LFS.

Native regression tests use temporary local repositories. Captured output in
`tests/data/rust/output-compatibility.json` comes from the Python reference and
covers help, version, status, YAML, and configuration output. Parallel operations
use bounded worker pools and collect each project's output separately. Tests
check that workers overlap, keep their own environment, and propagate exit codes.

Validation is ongoing; see [TODO.md](../TODO.md) for outstanding coverage. SSH
fixtures can delete shared test branches. Offline and LFS scenarios can change
host settings, so run those in disposable environments.
