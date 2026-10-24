# Contributing

## Rust migration branch

The native CLI is built from `src/`. Install the toolchain selected by
`rust-toolchain.toml`, then run:

```sh
cargo build --locked
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

Keep the Gherkin features unchanged. With the existing Python test dependencies
installed, pytest builds/selects the native CLI by default. An explicit executable
can select another native build:

```sh
export CLOWDER_EXECUTABLE="$PWD/target/debug/clowder"
export PYTHONPATH="$PWD"
python -m pytest tests/functional -q -m 'not ssh and not offline and not write and not lfs'
```

Use the interpreter from your test environment. This selection still uses public
Git repositories. Run offline, LFS, and write scenarios in disposable or serialized
environments as described in `AGENTS.md`. The Rust unit/integration tests use
temporary local repositories and do not need credentials or Python.
To compare behavior with Python, unset `CLOWDER_EXECUTABLE` and set
`CLOWDER_IMPLEMENTATION=python`.

See [native testing and coverage](docs/rust-testing.md), [the migration guide](docs/rust-migration.md) and
[native packaging instructions](docs/rust-installation.md). The legacy setup
below remains available for the Python behavior reference and published package.

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
