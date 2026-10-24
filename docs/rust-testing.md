# Native validation

## Local regression tests

Run `cargo test --locked` for native parser, CLI, filesystem, process, and
completion tests. These tests use temporary local repositories and require Git
and the macOS/Linux `script` and `ps` utilities for terminal input checks. They do not use
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
`CLOWDER_EXECUTABLE` and `PATH` pointing to the instrumented binary. Bash-based
launchers cannot reload CI startup exports and silently restore the ordinary
binary: coverage tests and the optional command run with `BASH_ENV` unset after
setup is loaded. For example, with the
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

The Linux native CircleCI job also runs this script with the 681 read-only BDD
cases and stores `build/coverage/rust/` as a build artifact. Release archives are
built separately, without coverage instrumentation. `run.txt` records the source
revision, working-tree status, tool versions, and exit status. Old reports are
removed before a new run, so failed validation cannot leave a stale report.

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

## Disposable Linux integration tests

Install Docker with Linux container support and use host Python 3.8 or later:

```sh
script/test-isolated build
script/test-isolated all
# Or select one suite, for example:
script/test-isolated write
script/test-isolated source-urls
```

The build uses current tracked and unignored repository files, so local edits are
included. It installs the pinned Rust toolchain and Python test requirements,
runs native tests, and builds the release executable. It does not copy `.git`,
host SSH keys, Docker sockets, or ignored build/test trees. Rebuild after source
changes. `--image` selects an image tag and `--reports` selects the output folder.

Each suite runs in a fresh container. A multi-suite run resolves the image tag
to one image ID at the start and records that ID in the report directory. Only `build/isolated/` is mounted for JUnit
and Cucumber reports. The container's Git configuration, fixtures, LFS filters,
and network interface are disposable. `NET_ADMIN` is granted only to the write
suite's local connectivity stub and the two suites that exercise offline behavior.
No command changes host Git configuration or host networking.

| Suite | Cases | Transport and isolation |
| --- | ---: | --- |
| `read` | 681 | Public HTTPS reads; no credentials. |
| `write` | 89 | Network disabled; Git URLs redirect to private bare copies. |
| `schema` | 211 | SSH-form URLs use private copies, populated by public HTTPS. |
| `schema-lfs` | 17 | Private Git copies; public HTTPS downloads for LFS objects. |
| `ssh-features` | 19 | Private copies, including fixture cleanup and branch changes. |
| `lfs` | 3 | Public HTTPS; LFS configuration changes stay inside the container. |
| `offline` | 51 | Disables/restores only container `eth0`. |
| `offline-write` | 12 | Private Git copies; the transport rejects requests while `eth0` is down. |
| `source-urls` | 28 | Network disabled; custom URLs use the private Felidae mirror. |

The eight writable/custom-URL fixture repositories have recorded branch/tag
objects in `tests/isolation/fixtures.json`; ephemeral `pytest*` branches are
excluded. Image construction verifies those objects and restores the recorded
refs. No private bare repository retains a public push destination. The SSH
adapter never invokes real SSH and permits receive-pack only for these private
fixtures in explicitly selected suites. Additional read-only upstreams and LFS
objects are fetched from public HTTPS, so those checks still depend on remote
availability. Python base-image and OS package updates can also affect the
container; this is a reproducible procedure, not a byte-identical image lock.

The custom-URL tests check URL construction and Git behavior against local data.
They do not validate Beanstalk credentials, connectivity, or its server key.
That live-service check remains pending, separately from these 1,111 cases.
The legacy `script/docker` copies host SSH keys; use `script/test-isolated` for
this validation path.
