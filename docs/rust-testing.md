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
