# Manifest cache benchmark

On this machine, a content-checked cache of schema-validated manifest data reduced
native `clowder --help` time by **23–30%**. The absolute saving was **1.6 ms for
8 projects** and **91 ms for 10,000 projects**. There was no clear improvement in
`status` across eight local repositories: Git work dominated that command.

These results support caching for large manifests and frequent startup-sensitive
operations. They do not show a meaningful benefit for ordinary Git commands in
small workspaces. The shipped CLI has not changed; all cache code ran in a
disposable source copy.

## End-to-end results

Measured on 2026-10-24 at source commit
`4596dcca765d819bc14d0b3a17c53b56b11872cf`, using release builds on an Apple M1 Max
(10 CPUs, 64 GiB RAM), macOS 27.0.1, Rust/Cargo 1.99.1. Each cell is the median of
31 fresh processes after three warmups, with the filesystem cache warm.

The baseline below is the unchanged CLI. A cache hit still reads and SHA-256
hashes the YAML, deserializes JSON, and resolves inheritance, current config,
project paths, and duplicate paths. It skips YAML parsing and schema validation.

| Projects | YAML bytes | Current `--help` | Cache hit | Time saved | Reduction |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 8 | 2,804 | 6.94 ms | 5.34 ms | 1.61 ms | 23.2% |
| 100 | 23,160 | 9.44 ms | 7.07 ms | 2.37 ms | 25.1% |
| 1,000 | 220,260 | 37.19 ms | 26.33 ms | 10.86 ms | 29.2% |
| 10,000 | 2,191,260 | 302.40 ms | 211.06 ms | 91.34 ms | 30.2% |

The 95% bootstrap intervals for time saved were 1.35–1.89 ms, 2.08–2.65 ms,
10.31–11.51 ms, and 88.92–93.43 ms, respectively. These intervals describe
variation within this run, not expected results on other machines.

For `status` over eight tiny local Git repositories, the medians were 564.46 ms
without caching and 559.55 ms with caching. The interval for time saved was
**−3.06 to 12.34 ms**, which includes zero. This experiment does not establish a
`status` speedup.

A second run reused the exact binaries without rebuilding and interleaved the
smallest and largest fixtures. It confirmed `--help` at 6.86 → 5.24 ms for
8 projects and 304.77 → 212.01 ms for 10,000. Its `status` interval also included
zero (−8.64 to 14.15 ms). As a process/startup control, `--version`, which does
not load the manifest, took 4.66 ms in the original binary and 4.50 ms in the
prototype binary.

## Cache alternatives

All rows below use the real `--help` command. The first row is the unchanged
binary; the remaining rows use the disposable prototype. Times are medians.

| Loading method | 8 projects | 100 projects | 1,000 projects | 10,000 projects |
| --- | ---: | ---: | ---: | ---: |
| Current CLI | 6.94 ms | 9.44 ms | 37.19 ms | 302.40 ms |
| JSON + content hash, revalidate schema | 6.44 ms | 8.56 ms | 29.85 ms | 233.22 ms |
| Validated JSON + content hash | 5.34 ms | 7.07 ms | 26.33 ms | 211.06 ms |
| Validated JSON + timestamp/size only | 5.37 ms | 7.05 ms | 25.23 ms | 204.93 ms |
| Content-hash cache miss, including write | 7.11 ms | 9.88 ms | 38.18 ms | 312.66 ms |

Caching only the parsed tree leaves schema construction and validation on every
invocation. Caching a validated tree removes both costs. The prototype retains
live model construction and path checks in either case; it does not serialize a
finished `Manifest` with potentially stale resolved paths or URLs.

Timestamp/size checks saved little extra time for small manifests. For the
largest fixture they saved about 6 ms beyond content hashing, but the correctness
probe demonstrated a stale result after an equal-length edit with restored mtime.
That is not an acceptable tradeoff for commands that act on repositories.

Misses cost extra: about 10 ms for the largest fixture in the main run and 7 ms
in the confirmation. The measured miss includes hashing, parsing, validation,
serialization, and an atomic cache-file replacement, with no `fsync`. Its cache
directory already exists; first-use directory creation is not included.

## Where time goes

Separate, fresh-process stage measurements help explain the result. These are
internal medians, not an additive decomposition of the command timings above.

| Stage | 8 projects | 10,000 projects |
| --- | ---: | ---: |
| Workspace/config discovery | 0.048 ms | 0.048 ms |
| Manifest selection and read | 0.036 ms | 0.389 ms |
| YAML parsing | 0.518 ms | 85.727 ms |
| Schema construction | 0.963 ms | 1.071 ms |
| Schema validation | 0.066 ms | 20.316 ms |
| Inheritance, model construction, path/duplicate checks | 0.194 ms | 182.736 ms |
| JSON serialization | 0.005 ms | 2.207 ms |
| JSON deserialization | 0.026 ms | 15.392 ms |
| SHA-256 of source bytes | 0.011 ms | 6.270 ms |

At 10,000 projects, most remaining time is in model construction and live path
resolution. JSON deserialization is also measurable. A binary representation or
caching additional declarative resolution might improve results, but neither
was measured here. The JSON stage measures the raw tree, including disposal of
the decoded value; command timings include the full cache envelope and I/O.

## Method and correctness

The [driver](../script/benchmark-manifest) builds the original release binary and
creates a private source copy. It injects the
[benchmark module](../script/benchmarks/manifest_cache.rs), separates schema
validation from the existing model constructor, and adds `sha2 = 0.10.9` only in
that copy. The copy retains the application's existing dependency resolutions;
its own lockfile records the extra hashing dependencies. Production source,
dependencies, and Gherkin files are unchanged.

Fixtures are synthetic manifests with 8, 100, 1,000, and 10,000 projects, including
defaults, sections, groups, a source, upstreams, and Git settings. Project
directories exist. The smallest fixture gets eight local Git repositories with
empty commits for `status`. No Git network access or remote writes are used.
Global/system Git configuration is excluded from the fixture commands.

Every timing launches a new process and includes process creation, CLI output
formatting, and shutdown. Timed stdout goes to `/dev/null`. Each round randomizes
mode and command order; the confirmation also interleaves fixture sizes. The
benchmark-only `load` action measures manifest/model loading without help-table
formatting or Git operations; its results are in the raw data.

Untimed checks passed for:

- Exact stdout, stderr, and exit-code equality across all measured modes,
  commands, and fixture sizes, including the original binary.
- Content-hash invalidation after a same-size edit with mtime restored. The
  timestamp/size-only mode deliberately fails this check by returning stale data.
- Applying a changed protocol configuration on a cache hit.
- Detecting a new duplicate path after a project symlink changes on a cache hit.
- Steady-state cache hits: cache files were not rewritten in any hit mode at any
  fixture size.

The analysis uses 10,000 paired bootstrap resamples of rounds, seed `20261024`,
to estimate the difference of medians. This was an ordinary workstation, not an
isolated performance lab. Load averages are recorded in the raw files; they
varied during both runs. No filesystem cache was purged. These measurements do
not cover cold disk access, network filesystems, Linux, populated worktrees,
shell completion, concurrent cache writers, or cache eviction.

The cache prototype is deliberately limited to controlled, disposable data. It
uses a separate directory for each run and fails on write errors so a fallback
cannot silently affect timing. It is not ready to ship as a production cache.

The smoke run, full run, confirmation, formatting checks, Python syntax checks,
local documentation links, and strict Clippy check of the prototype passed.
The Sphinx build could not start through Pipenv because the pinned Python 3.8.16
is missing on this workstation. Interpreter and dependency pins were unchanged.

## Reproduce

Requirements: the repository's Rust toolchain, Git, and Python 3.8 or later
(standard library only). Cargo may download build dependencies. Run from the
repository root with a new output directory:

```sh
mkdir -p build/benchmarks
set -o pipefail
python3 script/benchmark-manifest --samples 31 --warmups 3 \
  --output build/benchmarks/manifest-local \
  2>&1 | tee build/benchmarks/manifest-local.log
python3 script/benchmarks/analyze_manifest.py \
  build/benchmarks/manifest-local/results.json --confirm \
  2>&1 | tee build/benchmarks/manifest-local-analysis.log
```

The analyzer needs the binaries, fixtures, and caches from that run beside
`results.json`. Both commands leave application source unchanged. For a shorter
smoke run, use `--sizes 8,100 --samples 3 --warmups 1`. The source-patching step
fails explicitly if the manifest implementation no longer matches.

The recorded [raw measurements](benchmarks/manifest-cache-2026-10-24/results.json)
and [analysis/confirmation](benchmarks/manifest-cache-2026-10-24/analysis.json)
include individual samples, toolchain/hardware metadata, source fingerprint,
and binary hashes. Generated source, binaries, and fixtures remain under the
ignored `build/benchmarks/` directory.

## Proposed follow-up

If startup performance warrants the extra code, use a disposable per-user cache
of schema-validated declarative data, checked against the current source bytes.
Version the cache by parser/schema semantics and format, retain live config and
path resolution, and fall back to normal loading on missing, corrupt, incompatible,
or unwritable cache entries. Add bounded cleanup and concurrent-writer tests.
Do not create `.clowder` for caching: that directory already identifies the
manifest Git repository and affects workspace discovery and `init`.

The benchmark and the pending implementation decision are tracked in
[TODO.md](../TODO.md). No production caching is enabled by this experiment.
