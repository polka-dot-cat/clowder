# Swift CLI migration

The combined `rust-migration` branch contains the intended production Swift
implementation alongside the maintained Rust comparison and Python reference.
See the [implementation policy](implementations.md). It uses the same command grammar, manifest
schema, output captures, and unchanged Gherkin scenarios. It does not invoke
Python or Rust to implement commands. Git remains an external dependency.
The command scope and retained legacy limitations follow the
[native compatibility notes](rust-compatibility.md).

Swift was selected to support both the CLI and a future Mac app. All three
implementations remain available for correctness and performance comparisons.
Manifest caching remains deferred.

## Build and run

Use Swift 6.2 or later on macOS or Linux, with Git on `PATH`:

```sh
swift build --package-path swift -c release
swift/.build/release/clowder --version
swift/.build/release/clowder --help
```

The package declares macOS 13 or later. Local execution has been tested on
macOS 27, not on the oldest declared OS. Linux can bundle the Swift runtime:

```sh
swift build --package-path swift -c release --static-swift-stdlib
```

That Linux build still uses system C/C++ libraries; it is not fully static.
No runtime resource bundle is required: the shared schema and grammar are
embedded in the executable. Bash/Zsh completion wrappers in `completions/`
work with either native implementation.

## Implementation

- `swift/Sources/ClowderCore/` contains argument parsing, YAML/INI handling,
  schema validation, manifest inheritance, workspace discovery, Git operations,
  and shell completion.
- `swift/Sources/ClowderCLI/` is the executable entry point.
- `swift/Sources/CClowder/` is a small POSIX process adapter. It gives each child
  its own process group, working directory, environment, and output descriptors;
  signal handlers only set a lock-free flag. It supports terminal handoff,
  suspension/resumption, and escalation during cancellation.
- Bounded Swift task groups run project operations in parallel. Blocking process
  waits use a system dispatch queue so they do not block the cooperative Swift
  executor. Each project owns its output; parallel `forall` output spools to disk.
- YAML uses the libyaml event reader bundled with
  [Yams 6.2.2](https://github.com/jpsim/Yams/tree/a27b21e0c81c5bf42049b897a62aaf387e80f279).
  Swift resolves YAML 1.1 values, aliases, and merges; Yams emits saved manifests.
  The event reader preserves Python's duplicate-key behavior rather than using
  Yams's stricter mapping constructor. Parser limits match the native manifest
  contract: finite 64-bit numeric values, string keys, bounded depth/expansion,
  and no recursive aliases or unquoted timestamps.
- Schema validation compiles the draft-07 assertion vocabulary used by the
  checked-in schema. Unknown assertion keywords fail explicitly. This is a
  validator for Clowder's schema, not a general JSON Schema package. Schema
  rejection messages are less detailed than Rust's validator messages.

`swift/Package.resolved` pins the dependency revision. Dependency notices are in
[swift/THIRD-PARTY-NOTICES.md](../swift/THIRD-PARTY-NOTICES.md). Release signing,
publication, and a complete notice audit for redistributed static Swift runtime
libraries remain release prerequisites. Only Swift is planned for new native
release publication; retained Rust and Python code support comparisons.

## Validation

```sh
script/check-swift
swift test --package-path swift
# Reuse the existing native black-box regression tests without modifying them:
python3 script/test-native-compatibility swift/.build/debug/clowder
```

The black-box runner needs Cargo as a test tool. The Swift executable itself
has no Rust dependency. It runs the original 12 CLI and 7 process tests against
the supplied executable, including completion, large output, dirty-tree
protection, cancellation, and terminal job control.

The Swift Testing suite covers the captured Python YAML/INI corpus, output,
manifest inheritance, schema constraints, path resolution, parallel execution,
and failure codes. Each filesystem test owns a temporary workspace.

The existing Python environment can select Swift explicitly:

```sh
export PYTHONPATH="$PWD"
CLOWDER_IMPLEMENTATION=swift python -m pytest tests/functional/test_features.py \
  -q -m help -k empty_directory
# To select an already-built release binary:
CLOWDER_IMPLEMENTATION=swift CLOWDER_EXECUTABLE="$PWD/swift/.build/release/clowder" \
  python -m pytest tests/functional -q \
  -m 'not ssh and not offline and not write and not lfs'
```

Automatic Swift selection builds once before pytest workers start. Swift is
the harness default. Python reference selection is unchanged. The broad
read-only selection still reads public Git repositories.

Run the full matrix in disposable Linux containers, including offline, LFS,
write, SSH, and custom-URL scenarios:

```sh
python3 script/test-isolated build
python3 script/test-isolated all
```

Default reports go to `build/isolated/swift/`. Add `--implementation rust`
for the Rust image and `build/isolated/rust/` reports.

The Swift image uses Swift 6.4 and Python 3.10 on Ubuntu 22.04. It uses the same
pinned private fixture refs and transport isolation as the Rust image. No host
credentials, Git configuration, or networking are changed. Beanstalk's live
host-key/credential validation remains deferred.

Validation passed on macOS and Linux: all 10 Swift test functions (including
1,175 captured/generated YAML and INI cases), all 19 native black-box checks
on macOS, 18 existing empty-directory help scenarios, and the full unchanged
1,111-case isolated functional matrix. The full matrix also passed after the
final process/parser hardening changes; that run used four workers for the
681 read cases and serial execution for each remaining suite. All Gherkin
files remain unchanged.

The Linux release passed all 18 applicable native black-box tests in a Debian 12
container without a Swift installation; the extra Zsh test is macOS-only. The
[validation record](benchmarks/implementations-20261025/validation.json) gives
suite counts, image/binary fingerprints, and limits. Both local platforms are
arm64; remote CI and execution on the oldest declared macOS remain pending.

Like Rust, Swift preserves the captured output contract but does not duplicate
all of Python's presentation: `forall` omits the `>` command prefix and live
parallel progress display, and workspace metadata has one fewer blank line.

## Three-way benchmark

The [measured comparison](cli-implementation-benchmark.md) includes all three
implementations, raw timings, confidence intervals, and distribution tradeoffs.

Use an existing Python interpreter with the reference CLI's dependencies. The
driver builds optimized Rust and Swift binaries, checks output, then launches
fresh processes in randomized order. It uses only disposable local workspaces
and Git repositories, with warm filesystem caches and no manifest caching.

```sh
set -o pipefail
python3 script/benchmark-implementations --python /path/to/venv/bin/python \
  --samples 31 --warmups 3 --sizes 8,100,1000 \
  --output build/benchmarks/implementations-local \
  2>&1 | tee build/benchmarks/implementations-local.log
python3 script/benchmarks/analyze_implementations.py \
  build/benchmarks/implementations-local/results.json
```

Choose a new output directory for each run. Raw samples, toolchain/package
versions, source/binary fingerprints, fixture sizes, and execution order are
recorded in `results.json`; analysis adds paired bootstrap intervals. Python's
known Trio deprecation warning is suppressed. All other stderr and exit codes
must match. `forall` checks the selected project/status lines across Python and
the native ports; all other commands compare exact output bytes. Rust and Swift
always require exact byte agreement, including `forall`.

## Shared generated inputs

After changing `src/cli_spec.json` or `clowder/clowder.schema.json`, run:

```sh
script/update-swift-resources
script/update-swift-resources --check
```

`Embedded.swift` is generated; edit the shared JSON sources. `script/check-swift`
checks this relationship and lints handwritten Swift files without descending
into dependency/build directories. Gherkin files stay unchanged.
