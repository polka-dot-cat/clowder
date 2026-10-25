# Python, Rust, and Swift CLI benchmark

Rust was fastest in every measured workload on this machine. Swift provides a
large improvement over Python, but it did not improve performance over the
existing Rust port. Swift took **2.0–2.5 times as long** for manifest commands,
with a smaller **7–14%** difference for the local Git workloads.

For eight projects, `--help` took 6.48 ms in Rust, 14.06 ms in Swift, and
363.23 ms in Python. For 1,000 projects, it took 35.80 ms, 72.44 ms, and
11.77 seconds respectively. Those large Python differences include imports,
YAML/schema handling, model/path resolution, and output; they are not isolated
parser or language measurements. No implementation uses a manifest cache.

## Results

Measured on 2026-10-25 UTC (October 24 locally), at source commit
`704d19164e031d00ea7daa442ee01425aead4696`. Each value is the median of 31
fresh-process measurements after three warmups. Times are milliseconds.

| Workload | Projects | Python | Rust release | Swift release |
| --- | ---: | ---: | ---: | ---: |
| `--version`, empty directory | 0 | 228.40 | 4.44 | 8.54 |
| `--help`, empty directory | 0 | 229.07 | 4.56 | 8.96 |
| `--help` | 8 | 363.23 | 6.48 | 14.06 |
| `yaml` | 8 | 370.19 | 6.51 | 15.39 |
| `--help` | 100 | 1,298.53 | 9.17 | 20.09 |
| `yaml` | 100 | 1,362.57 | 10.67 | 25.77 |
| `--help` | 1000 | 11,770.99 | 35.80 | 72.44 |
| `yaml` | 1000 | 12,294.02 | 48.89 | 123.13 |
| `status`, local Git repositories | 8 | 731.80 | 531.04 | 570.32 |
| `forall -j1`, local Git repositories | 8 | 1,298.11 | 581.78 | 624.69 |
| `forall -j4`, local Git repositories | 8 | 1,221.92 | 180.26 | 204.95 |

All tested Swift/Rust differences had positive 95% paired bootstrap intervals
for “Swift time minus Rust time” within this run:

| Workload | Extra Swift time | 95% interval |
| --- | ---: | ---: |
| 8-help | 7.58 ms | 7.46–8.05 ms |
| 1000-help | 36.64 ms | 36.10–37.24 ms |
| 1000-yaml | 74.24 ms | 73.44–75.39 ms |
| local-status | 39.28 ms | 31.29–49.97 ms |
| local-forall-j1 | 42.90 ms | 34.50–50.39 ms |
| local-forall-j4 | 24.70 ms | 14.66–28.90 ms |

The relative startup difference is noticeable in short commands, while the
absolute difference for normal Git work is smaller. Four workers reduced this
short `forall` workload from 582 to 180 ms in Rust and from 625 to 205 ms in
Swift. Python's parallel run was 1,222 ms versus 1,298 ms with one worker; its
process/UI overhead remains part of the measurement. Longer shell commands or
network operations can behave differently.

## Implementation and distribution tradeoffs

Both native ports implement all 17 commands without a Python fallback, use the
shared grammar/schema, and pass the unchanged functional suite. Swift uses Yams's
libyaml reader with native YAML value resolution, a schema validator tailored to
the checked-in schema, Swift task groups, and a small C process adapter. The
custom validator needs maintenance when the schema's assertion vocabulary
changes and currently gives less detailed rejection messages than Rust.

The measured macOS files are 5,345,888 bytes (5.10 MiB) for Rust and 1,920,904 bytes
(1.83 MiB) for Swift. Swift uses macOS-provided Swift/Foundation libraries, so
these sizes do not measure total runtime footprint. The Linux Swift build with
`--static-swift-stdlib` is 75,125,496 bytes before stripping and 57,790,120 bytes
(55.11 MiB) after stripping. It still links system C/C++ libraries. That binary
passed the native tests on Debian 12 without a Swift installation. Linux speed
and a comparable Rust Linux package size were not measured here.

Measured performance favors keeping Rust for this port. Swift remains a viable
alternative if language familiarity or maintenance preferences outweigh its
extra startup cost, custom validator, and Linux runtime packaging work. This
comparison does not change the implementation selected by the existing test
harness or enable manifest caching.

## Method

Measured on an Apple M1 Max (10 CPUs, 64 GiB RAM), macOS 27.0.1, with Python
3.10.11, Rust 1.99.1, and Apple Swift 6.4. The Python environment retains the
repository's PyYAML 6.0, jsonschema 4.17.3, and Trio 0.22.1 dependencies. Python
3.8.16 was not available; no interpreter or dependency pins were changed. The
reference calls `yaml.safe_load`, which uses PyYAML's Python `SafeLoader`; it
does not select `CSafeLoader` even though libyaml is installed.

All three implementations run natively as arm64. Each workload has 31 measured
rounds after three warmups. Every round shuffles the implementation order and
starts a fresh process. Time includes process
creation, imports/runtime initialization, parsing, command work, output, and
shutdown. Timed stdout goes to `/dev/null`; stderr is captured. Untimed checks
require empty stderr and successful exit codes. Filesystem caches are warm. Builds and functional tests finished before the
measured run. These are workstation measurements, not results from an isolated
performance lab. The one-minute load average was 7.35 at the start and 10.11 at
the end; randomized interleaving reduces drift between implementations but does
not remove host-load effects.

Synthetic manifests contain 8, 100, and 1,000 projects, with defaults, sections,
groups, a source, upstreams, and Git settings. Project directories exist. The
8-project workspace gets eight local Git repositories with empty commits for
`status` and `forall`; there is no Git network access or remote mutation in the
benchmark. The `forall` workspace uses a `.clowder` directory and linked manifest
because Python requires that layout. The command is
`git rev-parse --verify HEAD >/dev/null`, using one or four workers.

Untimed checks compare exact stdout/stderr/exit codes for every workload except
Python's `forall` presentation. For `forall`, all modes must report the same
ordered project/status lines and succeed with empty stderr. Python's live
parallel progress display and sequential `>` command prefix differ from both
native ports; Rust and Swift must still match each other byte for byte. The
benchmark retains these UI costs in the timed commands. A known Trio import
deprecation warning is suppressed in Python; no reference code is changed.

The driver records raw times, execution order, source and binary fingerprints,
fixture hashes/sizes, package versions, and load averages. Analysis uses 10,000
paired bootstrap resamples of rounds to estimate differences of medians. The
intervals describe this run's variability, not performance on other machines.

These measurements do not cover cold disk access, large populated worktrees,
network latency, Linux timing, or every command. They measure these three
implementations, including their libraries and UI, rather than the languages
in isolation.

## Validation and remaining checks

Swift passed 1,175 captured/generated YAML and INI cases within its Swift
Testing suite, 19 native CLI/process tests on macOS and 18 on Linux (the Zsh test
is macOS-only), and all 1,111 unchanged functional scenarios. The functional
matrix ran in disposable Linux containers, including write, offline, SSH, LFS,
and custom-URL scenarios. The final run used four workers for its 681 read cases
and serial execution within the remaining suites. Automatic Swift selection in
pytest passed the 18 existing empty-directory help cases.

The benchmark's untimed output comparisons all passed. Source formatting,
generated resources, Python syntax, the C adapter's warning check, documentation
links, and the renamed-binary compatibility-runner check also passed. The Python
and Rust runtime sources and all Gherkin feature files remain unchanged.

Hosted macOS/Linux Swift CI is configured but has not run: pushing the branch is
blocked by the CLI token's missing `workflow` scope and an unavailable SSH signing
agent. Execution on x86_64 and the oldest declared macOS version, release
packaging/signing, static Swift runtime redistribution notices, and live
Beanstalk validation remain pending. See [the guide](swift-migration.md) and
[TODO.md](../TODO.md) for those follow-ups.

## Reproduce and inspect

The [driver](../script/benchmark-implementations) needs Cargo, Swift 6.2 or later,
Git, and a Python reference environment with the repository dependencies. Cargo
and SwiftPM may download dependencies during the build step. Use a new output
directory:

```sh
mkdir -p build/benchmarks
set -o pipefail
python3 script/benchmark-implementations --python /path/to/venv/bin/python \
  --samples 31 --warmups 3 --sizes 8,100,1000 \
  --output build/benchmarks/implementations-local \
  2>&1 | tee build/benchmarks/implementations-local.log
python3 script/benchmarks/analyze_implementations.py \
  build/benchmarks/implementations-local/results.json
```

- [Raw samples, fingerprints, versions, and fixture details](benchmarks/implementations-20261025/results.json).
- [Bootstrap comparisons](benchmarks/implementations-20261025/analysis.json).
- [Validation counts and Linux image/binary details](benchmarks/implementations-20261025/validation.json).
