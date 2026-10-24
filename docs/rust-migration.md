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

Manifest parsing retains PyYAML's YAML 1.1 booleans, octal numbers, aliases,
and merge keys. Saved manifests quote strings that would otherwise change type
when read by Python. Captured Python cases in
`tests/data/rust/yaml-compatibility.json` check this behavior. Manifest mapping
keys must be strings; numbers must be finite and fit a 64-bit integer or float.
Quote dates to use them as strings. Recursive aliases are rejected, nesting is
limited to 128 levels, and alias expansion is limited to 64 MiB.

The INI reader also keeps ConfigParser defaults, key interpolation, case-insensitive
option names, multiline values, and duplicate-key errors. Captured cases in
`tests/data/rust/config-compatibility.json` verify parsing and serialization.

Validation is ongoing; see [TODO.md](../TODO.md) for outstanding coverage. SSH
fixtures can delete shared test branches. Offline and LFS scenarios can change
host settings, so run those in disposable environments.

The [validation record](rust-migration-validation.md) lists native test results,
the isolation used for remote writes and offline checks, and startup measurements.
The [compatibility notes](rust-compatibility.md) describe output, parallel-command,
and parser differences, along with limitations retained from the Python CLI.

## Install from source

After checking out this branch, build an optimized binary or install it with Cargo:

```sh
cargo build --release --locked
./target/release/clowder --version
# Optional: install into Cargo's bin directory.
cargo install --path . --locked
```

The executable requires Git on `PATH` and a shell for `forall` and `repo run`.
It does not require the Python package or its dependencies. Keep the Python
environment only when running the existing BDD harness or comparing behavior.

The [native installation guide](rust-installation.md) also explains archive
installation, checksums, platform requirements, and `script/package-rust`.
