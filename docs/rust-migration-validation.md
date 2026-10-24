# Rust migration validation

Validation on October 24, 2026 covers all 1,111 unique cases from the unchanged
Gherkin suite across the environments below. The 28 explicit custom source-URL
cases use private fixture copies through an SSH-form transport, with networking
disabled. Actual Beanstalk host trust and account authorization remain deferred
under the owner's existing decision.

## Results

| Selection | Cases passed | Environment and limits |
| --- | ---: | --- |
| Read-only cases excluding SSH, offline, and LFS | 681 | Local macOS and hosted Linux job 3022, using public fixture URLs. The Actions matrix also passes this selection on Ubuntu and Intel macOS. |
| Write cases excluding offline | 89 | Native Linux release executable with private fixture copies and container networking disabled. Also confirmed against published GitHub fixtures by serialized native CircleCI job [3021](https://circleci.com/gh/polka-dot-cat/clowder/3021) at `0d8b2fe4`, with the registered write key. |
| Main LFS command cases | 3 | Native Linux release executable, public HTTPS Git/LFS, disposable container. |
| SSH-form schema cases excluding LFS and explicit Beanstalk URLs | 211 | Native Linux executable with a test SSH transport that serves private bare copies fetched over HTTPS. Validates URLs, refs, inheritance, and submodules; does not establish account SSH authorization. |
| Explicit project/upstream source-URL cases | 28 | Native Linux release executable at `5c68fc1a`, private GitHub Felidae mirror, read-only SSH-form transport, container network disabled. Checks the exact configured URLs and inheritance; does not contact Beanstalk. |
| LFS schema cases | 17 | Private fixture transport with read-only public HTTPS LFS downloads. |
| Other SSH-form command cases | 19 | Private fixture transport; temporary remote branches exist only inside the container. |
| Offline cases excluding remote writes | 51 | Public HTTPS setup, then disable/re-enable the container's `eth0`. |
| Offline remote-write cases | 12 | Private fixture transport rejects requests while container `eth0` is down. Verifies failure before local branch mutations. |

The first Linux runs used `8d7d9bb8`, with the offline harness repair in
`8b9f3935`. The schema, LFS, SSH-form, and offline selections were repeated
successfully at `ab1e9110`. Hosted job 3013 also passes all 89 write cases at
`ab1e9110`; job 3018 passes all 89 again at `692d09e5` in 32m51s. Each case
is counted once across reports. Earlier failed runs remain
in the evidence directory and do not add to the passed count.

The YAML compatibility follow-up passes all 681 read-only scenarios again in a
single macOS run. It also matches 86 captured PyYAML examples. The current local
suite has 21 Rust tests on macOS and 20 on Linux, including shell-safe Git aliases,
duplicate symlinked project paths, parent traversal through symlinks, INI
compatibility, sequential shell input, and native completion. The extra macOS
case checks Zsh completion; Linux checks Bash. All 23 applicable configuration and
46 `forall` scenarios also pass after those follow-ups.

A separate Debian runtime image containing Git and the release executable,
with no Python interpreter, passes all eleven native CLI integration tests at
`5c68fc1a` with its network disabled. These include local Git mutations,
concurrent workers, shell input, completion, and captured reference output.
The Cargo source package contains 35 files, builds after extraction, and passes
all 21 native tests on macOS from that extracted source. `cargo install --path`
from the extracted package also produces a working executable in an isolated
installation directory. Native
archives include the shell scripts and dependency notices tied to `Cargo.lock`.

## Evidence

On the development workstation, logs and JUnit/Cucumber reports are stored under:

```text
/Users/jrgoodle/projects/clowder-projects/clowder-transfer-backups/20261024T023608Z/service-preflight-uds_8n9d/
```

`rust-validation-summary-current.json` lists the all-passing reports and 1,111
unique case keys. The earlier `rust-validation-summary.json` retains the initial
1,083-case checkpoint. `rust-source-url-reports/` contains the 28 isolated
custom-URL reports; `rust-source-url-fixtures/` contains their transport and mirror.
`rust-isolated-reports/` contains container reports. Local read-only reports are
also in `build/junit/rust-*.xml`. The container transport helpers and build
snapshot are in `rust-container/`; they are test infrastructure, not part of
the native distribution. No host network interface or host LFS configuration
was changed by the container tests.

## Startup measurement

On this Apple Silicon workstation, ten measured runs after one warm-up gave:

| Command | Python median | Rust release median |
| --- | ---: | ---: |
| `clowder --version` | 326.36 ms | 5.34 ms |
| `clowder yaml` | 325.49 ms | 5.41 ms |
| `clowder status` | 319.96 ms | 4.81 ms |

The fixture contains two missing projects with explicit `main` defaults, so
these measurements cover startup, parsing, and local output, without network
work. They do not predict clone/fetch speed. Raw measurements are in
`rust-startup-benchmark.json` alongside the test evidence.

For a local `forall` workload, eight cloned repositories each ran
`git rev-parse --verify HEAD >/dev/null`. Seven measured runs after one warm-up
gave these medians on the same workstation:

| Implementation | One worker | Four workers |
| --- | ---: | ---: |
| Python reference | 1,991.98 ms | 1,959.57 ms |
| Rust release | 517.42 ms | 190.12 ms |

The Rust four-worker run was about 2.7 times faster than its one-worker run for
this workload. These measurements include command startup and status output;
they do not predict network performance. `rust-forall-benchmark.json` contains
the samples and `benchmark-rust-forall.py` contains the local fixture setup.

## Hosted CI

At `0d8b2fe4`, CircleCI Linux job
[3022](https://circleci.com/gh/polka-dot-cat/clowder/3022) passes all 681 read-only
cases with no failures or skips. Apple Silicon macOS job
[3023](https://circleci.com/gh/polka-dot-cat/clowder/3023) passes native unit and
integration tests, formatting, Clippy, and archive extraction. Both jobs publish
archives with SHA-256 files; the Linux job also verifies a Cargo source package.
Downloaded archives pass their checksums, include completion files and dependency
notices, and execute on macOS and an x86_64 Debian container respectively.

All four jobs in Actions run
[40106198988](https://github.com/polka-dot-cat/clowder/actions/runs/40106198988)
pass at the same commit. Each job runs 14 initialization, 667 other read-only,
and three LFS scenarios against Rust. The matrix covers Ubuntu and Intel macOS
with Python 3.8 and 3.11 for the harness. The documentation build also passes.
The logs identify the native executable in each test stage.

The BDD harness builds/selects Rust by default; Python comparison requires
`CLOWDER_IMPLEMENTATION=python`. Local checks pass for automatic native builds
with xdist, explicit Python comparison, and Python Git aliases. Collection still
finds all 1,111 cases. Harness coverage reports do not measure the Rust CLI;
collect native coverage separately before reactivating reporting integrations.

The same commit passes all 89 serialized write cases in CircleCI job
[3021](https://circleci.com/gh/polka-dot-cat/clowder/3021), with no failures or
skips in 33m16s. All required PR checks are successful at this implementation
checkpoint. Actual Beanstalk host trust and access, native release
publication/signing, and the unrelated deferred integrations remain separate
tasks in [TODO.md](../TODO.md).

## Native hardening follow-up (2026-10-24)

All four follow-up areas have a repeatable repository workflow in
[the native testing guide](rust-testing.md).

- The local native suite passes 28 tests on macOS (27 on Linux; Zsh completion
  is macOS-specific). Process tests cover Ctrl-C, SIGTERM, stubborn grandchildren,
  queued projects, captured Git commands, failed fetches, large parallel output,
  terminal input, and Ctrl-Z/foreground resume.
- `script/coverage-rust` passes the native suite and all 681 read-only BDD cases
  with LLVM instrumentation. The macOS run measured **89.89% line coverage**,
  **89.87% function coverage**, and **87.70% region coverage**, excluding test
  source files. `src/process.rs` reached 97.24% line coverage. This report covers
  local tests and the read-only selection; it is not a claim that every remote
  write/offline path was measured. Linux CI now collects the same selection and
  retains HTML, LCOV, JSON, and run metadata as build artifacts. Qlty and Codecov
  remain inactive.
- The checked-in Docker workflow passes all **1,111 unique unchanged Gherkin
  cases**, with no skips: 681 read, 89 private write, 211 schema, 17 schema LFS,
  19 SSH-form commands, 3 other LFS, 51 offline, 12 offline write, and 28 custom
  source URL cases. Each suite has a fresh container; writes use private copies.
  This retains the separate pending live Beanstalk validation.
- The checked-in parser corpus adds 538 YAML and 531 INI cases. Three further
  seeds (`1`, `42`, `8675309`) each pass 5,026 YAML and 5,019 INI cases: 30,135
  additional comparisons. The generator found and now covers duplicate YAML
  anchors, explicit signed integers, bracket-containing INI sections, malformed
  interpolation keys, and the ten-level interpolation boundary.
- The source package builds successfully with 46 intended files (1.4 MiB before
  compression), including the generated corpus and native test modules. All 28
  native tests also pass from the extracted package. The Sphinx HTML build
  succeeds; stale API references and unlisted-page warnings remain in the
  existing documentation backlog.

Local coverage artifacts are under `build/coverage/rust/`; container reports are
under `build/isolated/`. Build/test transcripts and the earlier coverage baseline
are retained in the existing `service-preflight-uds_8n9d` evidence directory.

The same eight-local-repository `forall` benchmark was repeated after process
hardening (one warmup and seven measured runs per setting):

| Implementation | One worker median | Four workers median |
| --- | ---: | ---: |
| Python reference | 1,962.19 ms | 1,916.33 ms |
| Rust release | 611.10 ms | 231.36 ms |

These final Rust medians are higher than the earlier 517.42/190.12 ms sample,
while remaining about 3.2× and 8.3× faster than Python at the same worker counts.
This small Git-heavy local workload includes the added process management; it
does not predict network-bound clone performance. Raw samples are retained as
`rust-forall-hardening-benchmark.json` beside the earlier benchmark evidence.
