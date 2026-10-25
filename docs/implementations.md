# Implementation policy

Swift is the intended production CLI and the starting point for a future Mac app.
Keep Rust and Python in the same checkout for correctness and performance
comparisons. This decision was made on 2026-10-25 after reviewing the
[three-way benchmark](cli-implementation-benchmark.md).

| Implementation | Role | Source |
| --- | --- | --- |
| Swift | Production CLI and future shared Mac app core | `swift/` |
| Rust | Maintained performance comparison and independent correctness check | `src/`, `Cargo.toml` |
| Python | Compatibility reference and historical performance baseline | `clowder/`, `setup.py` |

## Branch and release scope

The combined work uses `rust-migration` and [draft PR #642](https://github.com/polka-dot-cat/clowder/pull/642).
The branch name preserves the existing PR and review history. Its history includes
all four commits originally developed on `swift-migration`; do further work on
the combined branch. Keep the PR as a draft until validation and owner review
are complete. After it merges into `master`, use ordinary feature branches.

New native release publication is planned for Swift. Rust archives remain useful
local/CI comparison artifacts. Retaining source does not commit the project to
three package-manager distributions or release schedules. The existing Python
package remains the published legacy release; do not invoke `script/deploy` to
publish this migration. Swift packaging, signing, runtime notices, and minimum
OS checks are tracked in [TODO.md](https://github.com/polka-dot-cat/clowder/blob/rust-migration/TODO.md).

## Shared behavior

The existing Gherkin features, schema, parser captures, command grammar, and
benchmark fixtures define the common contract. Keep existing feature files
unchanged and add regression coverage for behavior changes. The grammar remains
at `src/cli_spec.json` and the schema at `clowder/clowder.schema.json`; their paths
do not assign ownership to a language. Run `script/update-swift-resources` when
either changes.

Maintain Rust for the shared CLI surface. Keep Python stable as the legacy
reference; change it when needed for a deliberate contract correction or
dependency compatibility, and record the change. New Mac UI features can remain
Swift-only. Document any CLI feature differences before benchmarking them.
Neither native executable may invoke the other implementations as a fallback.

Swift core APIs should return structured results/progress and support
cancellation. Keep terminal presentation and exit behavior in the CLI as that
separation is developed, so a future app can use the same core.

## Tests and CI

Pytest and `script/test-isolated` default to Swift. CI always selects the intended
implementation explicitly. `CLOWDER_EXECUTABLE` overrides native build selection;
set `CLOWDER_IMPLEMENTATION` to match it. Python mode requires that override to be
unset. The test header identifies the selected implementation.

| Check | Coverage |
| --- | --- |
| Actions `pytest (...)` | Swift functional cases on macOS/Linux with the existing Python harness matrix; retain required-check names |
| Actions `CI - Swift` | Swift native tests, formatting/resources, shared native process/CLI checks, and release builds |
| Actions `Python reference` | Python help scenarios, generated parser captures, and harness isolation checks |
| Actions `Python, Rust, and Swift shared behavior` | Common local help, version, YAML, status, and sequential/parallel `forall` output checks |
| CircleCI `rust` / `rust_macos` | Rust native checks, comparison archives, and Linux read-only BDD/coverage |
| CircleCI `test_write` | All 89 Swift write cases using private fixture repositories in a disposable container |

The required CircleCI write check keeps its name and queue, but no longer needs
GitHub write credentials for its test cases. Live Beanstalk verification remains
separate. Qlty and Codecov remain inactive.

Run the full isolated matrix when shared Git behavior changes and before release:

```sh
script/test-isolated build --implementation swift
script/test-isolated all --implementation swift
script/test-isolated build --implementation rust
script/test-isolated all --implementation rust
```

Default reports are separate under `build/isolated/swift/` and
`build/isolated/rust/`. The complete matrix includes network, LFS, offline, SSH,
and write scenarios; keep those side effects inside the documented isolation.

## Benchmark policy

Use `script/benchmark-implementations` to build release binaries and run all three
implementations against identical fixtures. It verifies output before timing,
randomizes implementation order, and records raw samples, toolchains, source
revision/digest, binary digests, and fixture sizes/digests. Its `--check-only`
mode runs the same output checks in CI without collecting performance samples.

Only compare behavior supported by every implementation in a workload. The
documented Python `forall` presentation difference is normalized narrowly;
Rust and Swift output must match exactly. Add explicit expectations for any
future intentional difference rather than silently weakening comparisons.

Keep historical results immutable. The initial measured runtime source is
`704d19164e031d00ea7daa442ee01425aead4696`; a measurement after changing Python
is a new reference revision, not the unchanged historical baseline. Record
optimization results for Rust and Swift separately. Hosted CI timing noise must
not determine correctness or block a merge; run controlled benchmarks explicitly.
