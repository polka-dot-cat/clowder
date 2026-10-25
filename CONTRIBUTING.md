# Contributing

## Implementations and shared tests

Swift is the intended production implementation. Rust remains a maintained
comparison implementation; Python remains the compatibility reference and test
harness. Keep them in the same branch and follow the
[implementation policy](docs/implementations.md).

With Swift 6.2 or later and Git installed:

```sh
script/check-swift
swift test --package-path swift
swift build --package-path swift -c release
```

Keep the existing Gherkin features unchanged. With the Python test dependencies
installed, pytest builds/selects Swift by default. Use explicit selection when
comparing implementations:

```sh
export PYTHONPATH="$PWD"
# Local help cases; no public repositories or credentials required.
python -m pytest tests/functional/test_features.py -q -m help -k empty_directory
CLOWDER_IMPLEMENTATION=rust python -m pytest tests/functional/test_features.py -q -m help -k empty_directory
CLOWDER_IMPLEMENTATION=python python -m pytest tests/functional/test_features.py -q -m help -k empty_directory
```

Use the interpreter from your test environment. `CLOWDER_EXECUTABLE` can point to
an existing native build; set `CLOWDER_IMPLEMENTATION` to its language as well.
Unset `CLOWDER_EXECUTABLE` when selecting Python. Native tests use temporary local
repositories. Run offline, LFS, and write scenarios in disposable environments:

```sh
script/test-isolated build
script/test-isolated all
# Rust comparison uses its own image and report directory.
script/test-isolated build --implementation rust
script/test-isolated all --implementation rust
```

For Rust changes, use the toolchain in `rust-toolchain.toml` and run
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test --locked`. See the [Swift guide](docs/swift-migration.md),
[Rust testing guide](docs/rust-testing.md), and
[shared benchmark procedure](docs/cli-implementation-benchmark.md).
The legacy setup below supports the Python reference and published package.

## Getting Started

Clone the repository:

```bash
git clone git@github.com:polka-dot-cat/clowder.git
cd clowder
```

Add the directory you cloned the repo into to `PYTHONPATH` in your shell profile. For example, if cloned into `$HOME/clowder`

```bash
export PYTHONPATH="$PYTHONPATH:$HOME/clowder/clowder"
```

## Building

Install `clowder` for local development

```bash
script/update
```

Remove `clowder`

```bash
script/clean
```

## Testing

The existing tests are all pytest-bdd functional tests located in [tests/features](tests/features).

## Pull requests

Any new functionality or bug fixes must have associated tests, in order to prevent future regressions.
