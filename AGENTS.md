# AGENTS.md

## Scope and project

This file applies to the full repository. Follow applicable parent instructions;
on this workstation, also read `/Users/jrgoodle/AGENTS.md`. Use clear, direct
technical English. Track follow-up work in [TODO.md](TODO.md).

Clowder manages multiple Git repositories from a `clowder.yml` or `clowder.yaml`
manifest. This branch builds the native Rust `clowder` CLI from `src/main.rs`.
The retained Python reference uses `clowder.app:main` and the `clowder-repo`
distribution. Keep `Cargo.toml`, `src/cli_spec.json`, and `clowder/__init__.py`
version declarations consistent when preparing a release. macOS and Linux are
the declared platforms; Windows support is incomplete.

Read [README.md](README.md), [CONTRIBUTING.md](CONTRIBUTING.md), and the relevant
command or schema documentation before changing behavior.

## Repository map

### Rust migration branch

The native CLI is in `src/`, with its build in `Cargo.toml` and `Cargo.lock`.
Follow [docs/rust-migration.md](docs/rust-migration.md). Keep the Python CLI as
the behavior reference and test-support library during migration. The Rust
executable must not invoke Python to implement commands. Preserve all current
Gherkin feature files.

Use `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test --locked`. Pytest builds and selects the native binary by default.
Set `CLOWDER_EXECUTABLE` to an absolute executable path to test an existing build.
For the Python reference, unset that variable and set `CLOWDER_IMPLEMENTATION=python`.
The existing network, SSH, remote-write, and LFS precautions below
still apply. `tests/rust_cli.rs` uses temporary repositories and captured Python
output in `tests/data/rust/` for local regression checks.

`src/cli_spec.json` embeds the legacy argument grammar and base help text;
workspace-specific help tables are generated in Rust. Git remains an external
runtime dependency. Each worker owns its subprocess working directory,
environment, and output; do not change the process directory or environment
inside parallel operations.

`completions/` contains native Bash and Zsh wrappers. They invoke the executable
with `CLOWDER_COMPLETE=1`; completion may read local manifests and refs but must
not run a command handler or contact a remote.

`script/coverage-rust` measures native implementation coverage locally.
`script/test-isolated build` and `script/test-isolated all` run the full functional
matrix in disposable containers without host credentials. See
[docs/rust-testing.md](docs/rust-testing.md) for suite limits and parser corpus generation.
Keep Qlty and Codecov inactive until the owner requests reactivation.

`script/package-rust` builds local archives without publishing. After dependency
changes, use `script/update-rust-licenses` to regenerate `THIRD-PARTY-NOTICES.md`;
see the installation guide for the pinned notice tool. Review new license types
and source-text hashes in `about.toml` with the dependency update.

| Location | Purpose |
| --- | --- |
| `clowder/app.py`, `clowder/cli/` | Command registration, arguments, and command handlers. |
| `clowder/util/app/` | Shared argparse-based CLI framework; commands use a nested `Meta` class. |
| `clowder/model/` | Manifest models for sources, defaults, sections, projects, and upstreams. |
| `clowder/controller/` | Resolve inherited settings, select projects, and coordinate Git operations. |
| `clowder/environment.py`, `clowder/config.py` | Find workspace paths and read/write the INI-style `clowder.config`. |
| `clowder/util/git/` | Git command wrappers, output parsing, and repository/ref models. |
| `clowder/util/parallel.py`, `clowder/util/tasks/` | Parallel command execution and task pools. |
| `clowder/clowder.schema.json`, `clowder/util/yaml.py` | Manifest schema, validation, serialization, and links. |
| `tests/features/` | Gherkin scenarios for pytest-bdd. |
| `tests/functional/` | Scenario bindings, Given/When/Then steps, fixtures, and test data. |
| `docs/`, `docs/rst/` | User guides and Sphinx API documentation. Some API pages are stale. |
| `script/`, `.github/workflows/`, `.circleci/` | Setup, tests, docs, release scripts, and CI definitions. |

`tests/old-scripts/` contains legacy shell tests. Use the pytest suite for new
coverage. `build/`, `temp/`, `dist/`, `*.egg-info`, caches, and
`README-processed.md` are generated or ignored content; do not edit them as source.

### Swift comparison branch

The independent Swift CLI is under `swift/`; read [docs/swift-migration.md](docs/swift-migration.md).
Use `script/check-swift`, `swift test --package-path swift`, and a release build.
`CLOWDER_IMPLEMENTATION=swift` selects it in the unchanged BDD harness; Rust
remains the default. Use `script/test-isolated ... --implementation swift` for
side-effecting integration tests. Keep Python/Rust runtime fallbacks out of the
Swift executable. `Embedded.swift` is generated from the shared grammar/schema
by `script/update-swift-resources`. Do not edit `.build/` dependency/build output.

## Environment and setup

`.python-version`, `Pipfile`, and `Pipfile.lock` select Python 3.8.16.
`setup.py` lists Python 3.8 through 3.11, while GitHub Actions selects 3.8 and
3.11. These declarations need alignment. Keep code compatible with the declared
range until a separate change updates it.

The project uses Pipenv and `setup.py`; there is no `pyproject.toml`. Once the
selected interpreter and Pipenv are available, the intended local setup is:

```sh
pipenv sync --dev
pipenv run python -m pip install --no-deps -e .
pipenv run python -m clowder.app --help
```

These commands were not runtime-verified during the initial documentation review:
Python 3.8.16 was missing and this checkout had no Pipenv environment. Report
setup failures rather than silently changing interpreter or dependency pins.
Keep `Pipfile`, `Pipfile.lock`, `requirements.txt`, and `setup.py` consistent when
changing dependencies. `requirements.txt` is the pinned set used by CI;
`pytest-bdd` is pinned to 4.0.2 in `Pipfile`.

Read scripts before running them. `script/update` calls setup/bootstrap, changes
the environment, and regenerates `requirements.txt`. Bootstrap can install system
tools and Git LFS hooks. `script/cibuild` changes system Git LFS configuration.
`script/clean` deletes build/test output and removes the Pipenv environment.
`script/deploy` creates a GitHub release and uploads to PyPI; it is a release
operation, not a build check. Use `tee` when saving command output so it remains
visible in the terminal.

## Tests and validation

Run commands from the repository root in the project environment. Start with a
small test selection, then run the relevant functional scenarios:

```sh
# Help scenarios that use an empty temporary directory.
pipenv run python -m pytest tests/functional/test_features.py -q -m help -k empty_directory

# Broader functional run; still needs network access for example repositories.
pipenv run python script/test --markers 'not ssh and not offline and not write and not lfs' --exitfirst
```

`script/test` writes coverage, JUnit, and Cucumber reports under `build/` and uses
`temp/` as pytest's base temporary directory. Treat `temp/` as disposable. For a
focused run, use `python -m pytest` with a test path, `-m`, or `-k`. Add `--parallel`
to the wrapper only when the selected fixtures can run independently.

Test markers describe behavior, not complete isolation guarantees:

- `offline` scenarios can disable the host network through
  `clowder/util/connectivity.py`; use a disposable, isolated test environment.
- `ssh` scenarios require suitable SSH access. `write` scenarios can change remote
  branches. SSH fixture cleanup can also delete remote `pytest*` branches.
- `lfs` scenarios can uninstall Git LFS hooks and filters from global/system Git
  configuration. Run them only in an isolated environment.
- Many unmarked scenarios clone public example repositories. Excluding
  `internet` does not produce a reliable local-only test selection.
- `test_schema_validation.py` binds integration scenarios that run Git operations;
  it is not a standalone JSON Schema unit test suite.

Add regression coverage for behavior changes, as required by `CONTRIBUTING.md`.
Register new markers in `pytest.ini` because strict marker checking is enabled.
For documentation-only changes, check links and the diff; a full integration run
is not required. State which checks ran and any limits on validation.

## Change conventions

- Follow the local Python style: four spaces, type hints, `pathlib.Path`, and
  existing docstrings. `setup.cfg` permits 160 columns but `.pylintrc` uses 120;
  keep edits readable and avoid unrelated formatting changes.
- Keep CLI parsing in handlers, manifest representation in models, and Git work
  in controllers/utilities. Reuse `CONSOLE`, `Format`, and existing Git wrappers.
- Register a new top-level command in both `clowder/cli/__init__.py` and
  `ClowderApp.Meta.subcommands`. Update help, `docs/commands.md`, and scenarios.
- For manifest changes, update the schema, models/resolution, syntax reference,
  and fixtures together. Cover defaults, section overrides, project overrides,
  upstreams, and both YAML extensions where relevant.
- `ENVIRONMENT`, `CLOWDER_CONTROLLER`, and `SOURCE_CONTROLLER` are module-level
  state. Imports can capture the current workspace and resolve repositories.
  Account for this in tests that change directories or create multiple workspaces.
- Exercise Git mutations in temporary repositories. Preserve existing dirty-tree
  checks and test branch, tag, detached-HEAD, submodule, and upstream behavior
  affected by the change.
- Edit `README.md`; `setup.py` generates `README-processed.md`. For docs changes,
  use `pipenv run sphinx-build -b html docs build/docs/html` after setup.
  `script/docs` also rewrites API stubs and currently reads `$1` under `set -u`;
  inspect it before use.
