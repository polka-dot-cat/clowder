# Native validation

## Local regression tests

Run `cargo test --locked` for native parser, CLI, filesystem, process, and
completion tests. These tests use temporary local repositories and require Git
and the macOS/Linux `script` utility for terminal input checks. They do not use
credentials, public repositories, or Python. `cargo clippy --all-targets -- -D warnings`
and `cargo fmt --check` provide the remaining native checks.

## Native coverage

Install the pinned development tool and the LLVM tools for the repository's Rust
toolchain:

```sh
cargo install cargo-llvm-cov --version 0.9.1 --locked
rustup component add llvm-tools-preview
script/coverage-rust
```

The script instruments Rust with LLVM, including native CLI processes launched
by tests. It writes a summary, JSON, LCOV, and HTML under `build/coverage/rust/`.
Open `build/coverage/rust/html/index.html`. Profiles and instrumented builds use
`target/rust-coverage/`, separate from ordinary builds. Each run clears the old
profiles, so the result describes that run's selection. Test source files are
excluded from the report. Branch coverage requires nightly tooling and is not
reported by this stable-toolchain command.

An optional command runs after the local native tests and inherits
`CLOWDER_EXECUTABLE` pointing to the instrumented binary. For example, with the
existing Python test environment active:

```sh
script/coverage-rust -- python -m pytest tests/functional -q \
  -m 'not ssh and not offline and not write and not lfs'
```

That selection reads public Git repositories. The usual offline, LFS, and remote
write restrictions still apply to any supplied command. A failed command fails
the script; it does not produce a report labeled as a successful run. Set
`CARGO_LLVM_COV_BIN` to use a tool installed outside `PATH`. The script chooses
`rustup which rustc` unless `RUSTC` is set, so LLVM tools match the compiler even
when Homebrew tools appear first on `PATH`.

The legacy pytest coverage report measures Python harness/reference code, not
the native CLI. No coverage command here sends a report to Qlty, Codecov, or any
other service. Those integrations remain inactive.

Tool reference: [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov).

## Differential parser cases

`tests/data/rust/` contains the original hand-picked reference captures plus
538 generated YAML cases and 531 generated INI cases (seed `20261024`). Native
unit tests compare values, rejection of invalid input, YAML serialization round
trips, INI interpolation errors, and INI serialization with those expectations.
The Python oracle uses PyYAML 6.0 and ConfigParser from the retained test environment.

```sh
# Run with the existing Python 3.8–3.11 test environment, not system Python 3.14.
python script/generate-rust-compatibility --check
# To capture another bounded, deterministic sample without replacing the corpus:
python script/generate-rust-compatibility --seed 42 --count 5000 \
  --output build/compatibility/42
CLOWDER_COMPATIBILITY_DIR="$PWD/build/compatibility/42" \
  cargo test --locked --bin clowder matches_
```

A generated sample stays inside the documented manifest value contract. Dates,
nonfinite values, oversized integers, non-string mapping keys, binary values,
and recursive aliases are not counted as Python-compatible cases. The fixtures
record how many generated values were outside that contract. Inputs are small
and bounded; this is deterministic differential testing, not exhaustive fuzzing.

