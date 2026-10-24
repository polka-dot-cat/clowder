# Rust migration validation

Validation on October 24, 2026 covers 1,083 unique cases from the unchanged
1,111-case Gherkin suite. The 28 explicit Beanstalk source-URL cases remain
deferred under the owner's existing host-key decision.

## Results

| Selection | Cases passed | Environment and limits |
| --- | ---: | --- |
| Read-only cases excluding SSH, offline, and LFS | 681 | Native macOS executable and hosted Linux job 3011, public fixture URLs. The macOS YAML follow-up and hosted job each pass all 681 cases in a single run. |
| Write cases excluding offline | 89 | Native Linux release executable, private local fixture copies, container network disabled. No public repository writes. |
| Hosted write confirmation (same 89 cases) | 89 | CircleCI job [3009](https://circleci.com/gh/polka-dot-cat/clowder/3009), native executable at `408a2f4b`, published GitHub fixtures and the registered write key. All passed with no failures or skips in 33m34s. Counted once in the total above. |
| Main LFS command cases | 3 | Native Linux release executable, public HTTPS Git/LFS, disposable container. |
| SSH-form schema cases excluding LFS and explicit Beanstalk URLs | 211 | Native Linux executable with a test SSH transport that serves private bare copies fetched over HTTPS. Validates URLs, refs, inheritance, and submodules; does not establish account SSH authorization. |
| LFS schema cases | 17 | Same isolated transport, with read-only public HTTPS LFS downloads. |
| Other SSH-form command cases | 19 | Private fixture transport; temporary remote branches exist only inside the container. |
| Offline cases excluding remote writes | 51 | Public HTTPS setup, then disable/re-enable the container's `eth0`. |
| Offline remote-write cases | 12 | Private fixture transport rejects requests while container `eth0` is down. Verifies failure before local branch mutations. |

The first Linux runs used `8d7d9bb8`, with the offline harness repair in
`8b9f3935`. The schema, LFS, SSH-form, and offline selections were repeated
successfully at `ab1e9110`. Hosted job 3013 also passes all 89 write cases at
`ab1e9110`. Each case is counted once across reports. Earlier failed runs remain
in the evidence directory and do not add to the passed count.

Eight Rust tests pass on macOS and Linux at the original checkpoint: four unit
tests and four native CLI integration tests. These check manifest inheritance, parsing, captured Python
output, concurrent workers, exit codes, and validation before mutations.
`cargo fmt --check` and strict Clippy validation pass. A Cargo source package
builds successfully after extraction.

The YAML compatibility follow-up passes all 681 read-only scenarios again in a
single macOS run. It also matches 86 captured PyYAML examples. The current local
suite has 21 Rust tests on macOS and 20 on Linux, including shell-safe Git aliases,
duplicate symlinked project paths, parent traversal through symlinks, INI
compatibility, sequential shell input, and native completion. The extra macOS
case checks Zsh completion; Linux checks Bash. All 23 applicable configuration and
46 `forall` scenarios also pass after those follow-ups.

A separate Debian runtime image containing Git and the release executable,
with no Python interpreter, passes all eight native CLI integration tests at
`f646ccac` with its network disabled. These include local Git mutations,
concurrent workers, shell input, and captured reference output.

## Evidence

On the development workstation, logs and JUnit/Cucumber reports are stored under:

```text
/Users/jrgoodle/projects/clowder-projects/clowder-transfer-backups/20261024T023608Z/service-preflight-uds_8n9d/
```

`rust-validation-summary.json` lists reports and unique case counts.
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

## Remaining checks

Hosted native write CI has passed. At `692d09e5`, CircleCI Linux job
[3015](https://circleci.com/gh/polka-dot-cat/clowder/3015) passes all 681 read-only
cases with no failures or skips. Apple Silicon macOS job
[3016](https://circleci.com/gh/polka-dot-cat/clowder/3016) passes native unit and
integration tests, formatting, Clippy, and archive extraction. Both jobs publish
archives with SHA-256 files; the Linux job also verifies a Cargo source package.
The downloaded macOS archive passes its checksum and runs on this workstation.

The BDD harness now builds/selects Rust by default, including in the existing
Actions matrix; Python comparison requires `CLOWDER_IMPLEMENTATION=python`.
Local checks pass for automatic native builds with xdist, explicit Python
comparison, and Python Git aliases. Collection still finds all 1,111 cases.
Hosted verification of that default, including Intel macOS runtime tests, is
pending. Keep the PR in draft until the remaining checks in [TODO.md](../TODO.md)
are complete.
