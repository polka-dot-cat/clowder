# TODO

Initial backlog from repository review on 2026-10-15, at commit `1438fc8b`.
Priorities below are proposed. Findings come from source and configuration
inspection; application behavior and CI were not tested. The pinned Python
3.8.16 is not installed here, and this checkout has no Pipenv environment.

Check an item off only after its completion criteria are met. Add regression
coverage for behavior fixes. Abstract methods and pytest-bdd scenario bodies that
contain `pass` are not automatically missing implementations.

## Swift implementation comparison

Follow [the Swift comparison guide](docs/swift-migration.md). Keep the Swift
implementation alongside Rust while comparing compatibility and performance.

- [x] Add a native Swift CLI with the shared grammar/schema and no Python/Rust runtime fallback.
- [x] Preserve Python parser/output captures and pass native CLI/process regression checks.
- [x] Build and run Swift tests on macOS and isolated Linux.
- [x] Complete the unchanged 1,111-case functional matrix against Swift.
- [ ] Benchmark Python, Rust release, and Swift release with identical local workloads.
- [ ] Review measured tradeoffs before choosing the implementation to ship.
- [ ] Expand output captures for workspace metadata and `forall` UI. Both native
  ports omit Python's extra metadata blank line, `>` command prefix, and live
  parallel progress display; benchmark checks account for those differences.

## Rust migration

Follow [the migration plan](docs/rust-migration.md). Work in `rust-migration`
and keep PR #642 as a draft until compatibility and distribution checks pass.

- [x] Add the native Cargo binary, manifest schema validation and inheritance,
  workspace discovery, and command dispatch without a Python runtime fallback.
- [x] Route the existing Python harness to a selected native executable while
  preserving the Python reference path and all Gherkin files.
- [x] Capture representative Python output and add native regression tests for
  formatting, parallel process isolation, exit codes, and dirty-tree protection.
- [x] Pass the existing read-only functional suite against the Rust executable:
  681 unique cases pass across local runs after harness repairs.
- [x] Validate schema/source inheritance, recursive submodules, and LFS in
  disposable containers. See the [validation record](docs/rust-migration-validation.md)
  for transport limits. All 1,111 cases pass across the selected environments,
  including 28 custom-URL cases against private mirrors. Retain the separate
  pending Beanstalk host-key and live-service check.
- [x] Validate remote-write behavior in isolation and through serialized CI.
  All 89 cases pass in the isolated Linux environment and native CircleCI job
  [3021](https://circleci.com/gh/polka-dot-cat/clowder/3021), with no skips.
- [x] Validate all 63 offline scenarios in disposable environments. Repair the
  Python harness gateway-output capture so these scenarios can run on Linux.
- [x] Build and test the native CLI on macOS and Linux in hosted CI. Jobs
  [3022](https://circleci.com/gh/polka-dot-cat/clowder/3022) and
  [3023](https://circleci.com/gh/polka-dot-cat/clowder/3023) pass and publish native archives.
- [x] Add native installation/release instructions and local archive packaging;
  compare startup and parallel execution with the Python reference. See the
  [native installation guide](docs/rust-installation.md) and validation record.
  Release publication and signing remain separate tasks.
- [x] Audit remaining behavior and output differences before removing the Python
  CLI from the distribution. Keep Python support needed by the test harness.
  The [compatibility notes](docs/rust-compatibility.md) record the current audit.
- [x] Add native Bash and Zsh completion to replace Python `argcomplete`
  integration, including local project/group/version/ref candidates.
- [x] Validate Intel macOS runtime behavior. Both native Actions jobs in
  [run 40106198988](https://github.com/polka-dot-cat/clowder/actions/runs/40106198988)
  pass all 684 cases, including LFS, on `macos-15-intel`.

- [x] Harden native interruption, child cleanup, failed Git commands, and large parallel output.
- [x] Add reproducible local LLVM coverage for native Rust; keep Qlty/Codecov inactive.
- [x] Check isolated container and private fixture validation helpers into the repository.
- [x] Expand deterministic YAML/INI differential cases against the Python reference.
- [x] Benchmark manifest parsing and persistent cache options in disposable builds.
  See the [results](docs/rust-manifest-benchmark.md): validated JSON with content
  hashing cuts `--help` time by 23–30% in the synthetic fixtures; the small-workspace
  `status` difference is within timing noise. No production cache is enabled.
- [ ] **Deferred:** revisit manifest caching only if startup becomes noticeable.
  The owner deferred it after the benchmark. If adopted, use a versioned per-user cache checked against source contents, retain
  live config/path checks, and test invalidation, corrupt/unwritable entries,
  concurrent writers, and cleanup. Do not create `.clowder` for this cache.

## Before the backlog: transfer the repository

Follow the [transfer plan](docs/clowder-repository-transfer-plan.md) and
[progress record](docs/clowder-repository-transfer-status.md). The checks below
are separate from the initial review and were updated on 2026-10-24.

- [x] Save the Clowder Git mirror, LFS objects, release assets, and settings inventory.
- [x] Transfer `felidae` to `polka-dot-cat` and verify its ID, branches, tags, and redirect.
- [x] Prepare canonical clone URLs and post-transfer project links locally.
- [x] Validate the preparation with CLI help and 28 focused test cases.
- [x] Count the impact of the deleted forks: 97 collected cases depend on them;
  reproduce a missing-repository failure without remote writes.
- [x] Provision `polka-dot-cat/djinni`, `gyp`, and `sox` from their original
  upstreams; set GYP's `fork-branch` to
  `bd11dd1c51ef17592384df927c47023071639f96` and verify its upstream fast-forward.
- [x] Publish the restored-fork names in `misc-clowder-tests` and
  `clowder-examples`; update local Clowder fixtures and assertions. All 97 affected
  cases plus a new saved-defaults regression pass, including HTTPS and SSH coverage.
- [x] Transfer `ash` and publish canonical submodule URLs in Ash, Mu, and Felidae;
  verify recursive clones and preserve tags and unrelated refs.
- [x] Stage the remaining Cats and clowder-examples manifest names on migration
  branches and prepare their matching Clowder assertions in draft PR #641.
- [x] After PR #641 merged, fast-forward the active Cats and clowder-examples
  manifest branches to the [recorded commits](docs/clowder-repository-transfer-status.md#cats-and-felidae-migration).
  Verify all published refs and preserve unrelated branches and tags.
- [x] Validate hosted CI against the published manifests after removing the
  temporary fixture pins and URL overrides. All four Actions jobs pass at
  `ab1e9110` in [run 40100795242](https://github.com/polka-dot-cat/clowder/actions/runs/40100795242);
  CircleCI native read-only and write checks also pass.
- [ ] Verify Beanstalk's server host key, then check SSH access and run the 28
  explicit source-URL cases against the actual service. Retain Beanstalk; the owner confirmed the account
  public key is already registered and chose to leave host verification pending.
- [x] Validate the repaired Actions baseline. At `6607ba77`, all four Ubuntu/macOS
  and Python 3.8/3.11 jobs pass all 684 cases, including LFS. The documentation
  build passes too. The [preflight record](docs/clowder-transfer-preflight.md)
  links the run and its verified reports. The longest job needed 50 minutes.
- [x] Check Actions repository eligibility and effective permissions on existing
  organization repositories; confirm Pages has no verified domain on either account.
- [x] Inventory service failures and disable the obsolete Requires.io webhook
  with the owner's approval; remove its documentation badge.
- [x] Disable the Read the Docs webhook and remove its build badges at the owner's
  request. Retain the project and build configuration for later reactivation.
- [x] Restore CircleCI's GitHub configuration-fetch authorization. Pipeline `1578`
  loads the configuration and publishes the required status after the owner signed in.
- [x] Restore CircleCI's checkout/deploy key. The owner recreated it; GitHub
  confirms read-only deploy key `167499686`, titled `CircleCI`.
- [x] Repair CircleCI's active write job to avoid the retired Code Climate
  reporter, select the additional SSH key, check write access before tests, and
  serialize remote branch mutations. All 89 cases pass against isolated local copies.
- [x] Register the additional CircleCI write key with GitHub. The owner added
  it, and its public key is verified in `JrGoodle`'s authentication-key list.
  The working read-only checkout key is separate.
- [x] Verify the hosted `test_write` rerun after key registration. Job `2982`
  passed all 89 cases with no failures or skips; its GitHub required status is
  successful. See the [preflight record](docs/clowder-transfer-preflight.md#hosted-circleci-write-validation).
- [x] Defer Qlty and Codecov at the owner's request. Disable the legacy Code
  Climate webhook, keep coverage uploads disabled, and remove their badges.
  Track reactivation separately below; neither service blocks the transfer.
- [x] Confirm CircleCI's GitHub OAuth organization access to `polka-dot-cat`.
  The owner confirmed access is granted on October 24.
- [x] Verify a new hosted CircleCI `test_write` run under `polka-dot-cat`.
  Job `3009` passes all 89 cases against the published manifests using the
  native CLI. The unused Codecov orb and upload step have been removed;
  Codecov remains inactive and organization security is unchanged.
- [x] Audit canceled CircleCI job `2988`: permanent fixture refs are unchanged.
  Remove its four temporary prune branches with ref leases and verify cleanup.
- [x] Defer `clowder.cat` DNS, Pages, and domain-verification work at the owner's
  request. Website availability is not a transfer prerequisite; retain its
  current configuration and track restoration separately below.
- [x] Transfer `clowder`, update the local remote, and merge PR #641. The owner
  elected to merge before the latest checks completed. Verify repository identity,
  refs, releases, issue/PR numbers, collaborators, protection, keys, and settings.
- [ ] Merge the CI cleanup and complete the remaining transfer verification checks.

## Deferred website work — after transfer

- [ ] **Restore and verify `clowder.cat` under `polka-dot-cat`.** Review the
  retained `gh-pages` source and custom-domain configuration, update the
  `www.clowder.cat` CNAME from `jrgoodle.github.io` to `polka-dot-cat.github.io`,
  and verify `clowder.cat` in the organization's Pages settings with GitHub's
  supplied TXT record. Confirm a Pages deployment, HTTPS/certificate coverage,
  and the expected site at both the apex and `www` hostnames. Website repair
  can wait until after the repository move; availability is not required to
  complete the transfer.

## Deferred reporting integrations — after transfer

Both services stay inactive until their reactivation is requested. Keep generating
coverage files and storing them as CI artifacts in the meantime.

- [ ] **Set up Qlty under `polka-dot-cat`.** Connect the organization's GitHub
  app to the transferred Clowder repository, review the existing hosted project,
  and migrate the analysis configuration. Add a supported coverage uploader;
  account for reports from Actions and CircleCI without duplicate matrix uploads.
  Verify analysis and coverage for the correct default-branch and PR commits
  before restoring badges or enabling quality/coverage gates. Replace the
  retired Code Climate reporter and review removal of its disabled webhook.

- [ ] **Reactivate Codecov under `polka-dot-cat`.** Configure organization and
  repository access, replace the old disabled upload integration with a supported
  uploader and authentication, and define how retained CI reports are combined.
  Verify default-branch coverage and PR comparisons against the uploaded commit
  before restoring its badge or enabling coverage checks. Review overlap with
  Qlty if both services are enabled.

  For either service, collect Rust CLI coverage separately from Python harness
  coverage. Python coverage from a native BDD run does not measure the Rust CLI.

## First: establish a repeatable development baseline

- [ ] **Align Python support and dependency setup.** `.python-version` and Pipenv
  select 3.8.16; `setup.py` lists 3.8–3.11; GitHub Actions selects 3.8/3.11;
  Docker selects 3.8.5; Read the Docs now selects 3.11; AppVeyor selects 3.6–3.8.
  Choose the supported range, add package `python_requires`, and align these
  files. Verify installation, CLI help, and test collection in a fresh environment.
  Preserve the `pytest-bdd==4.0.2` constraint until migration is tested, and check
  uses of `pkg_resources` and `trio.MultiError` when updating dependencies.

- [ ] **Make the test setup independent of personal infrastructure.**
  [Test utilities](tests/functional/util/clowder.py) clone named external repos
  and can delete remote `pytest*` branches during SSH setup.
  [Network steps](tests/functional/when/when.py) disable host networking.
  LFS scenarios can change global/system hooks and filters.
  Add local bare-repository fixtures and simulated connection failures where
  possible; isolate the remaining integration tests. Document a test selection
  that needs neither credentials nor changes to host networking or remote repos.

- [ ] **Establish and record a CI baseline.** In
  [.circleci/config.yml](.circleci/config.yml), only `test_write` is enabled in
  the workflow; offline, parallel, and combined coverage jobs are commented out.
  [GitHub Actions](.github/workflows/ci-pytest.yml) excludes SSH/offline tests; its
  action versions and setup were repaired during the transfer preflight. Finish
  validating retained jobs, restore or explicitly retire disabled coverage, and
  record results for the chosen platform matrix. Fix or retire
  [AppVeyor](appveyor.yml), which uses an incomplete
  `pip3 install -e` command and an absent `examples/cats` directory.

## Next: fix incomplete or inconsistent behavior

- [ ] **Complete submodule cleanup and update handling.**
  [CleanCommand](clowder/cli/clean.py) accepts `--recursive` but does not pass it
  on. [ProjectRepo.clean](clowder/controller/project_repo.py) changes the parent
  repository before raising `NotImplementedError` for `submodules=True`.
  `ProjectRepo.herd` only calls `submodule_update` when `recursive` is true, while
  [resolved settings](clowder/controller/resolved_git_settings.py) distinguish
  `submodules: true` from `submodules: recursive`. Define and test nonrecursive
  and recursive behavior with nested local submodules, including error paths.

- [ ] **Reconcile existing remotes during herd.** The `configure_remotes`
  decorator in [project_repo.py](clowder/controller/project_repo.py) is a no-op,
  and `Remote._compare_remote_url` in [remote.py](clowder/util/git/model/remote.py)
  is commented out. Herd creates missing remotes but does not resolve the cases
  described by these TODOs. Define behavior for a renamed remote, a matching name
  with a different URL, and project/upstream collisions; add local-repo tests.

- [ ] **Handle manifest symlinks outside the workspace in status output.**
  [ClowderRepo.status](clowder/controller/clowder_repo.py) calls `relative_to`
  on the resolved target without handling an external target, as its FIXME notes.
  Display a valid path or a clear error. Cover internal, external, broken, and
  ambiguous `.yml`/`.yaml` links without an unhandled traceback.

- [ ] **Validate configured projects against actual project/group choices.**
  [Config.__init__](clowder/config.py) passes `self.projects` into
  `_validate_config_projects_defined`, which then checks the same values against
  themselves. Validate against resolved manifest choices and report all unknown
  entries. Cover valid groups, unknown names, and cleared settings in
  [config.feature](tests/features/config.feature).

- [ ] **Remove duplicate exception definitions.**
  [error.py](clowder/util/error.py) defines `ExistingFileError`,
  `ExistingSymlinkError`, `MissingFileError`, and `UnknownArgumentError` twice.
  The later definitions replace the versions derived from `ClowderError`.
  Confirm the intended hierarchy, keep one definition of each, and verify CLI
  error handling and relevant catches.

- [ ] **Define and test saved-version names.**
  [SaveCommand](clowder/cli/save.py) lowercases names and replaces `/` with `-`,
  but still has a validation TODO. Specify allowed names and collision handling,
  then validate before creating files. Preserve or explicitly document the
  slash-to-dash behavior covered by [save.feature](tests/features/save.feature).

## Strengthen regression coverage

- [ ] **Replace placeholder output checks with assertions.**
  `then_command_printed_branch_type` in [output.py](tests/functional/then/output.py)
  contains only `pass`. Status and diff scenarios also contain commented output
  checks. Add active scenarios that assert local/remote branch lists, dirty state,
  and diff output after stripping terminal escapes. Verify the assertions fail
  for incorrect output.

- [ ] **Cover parallel execution and unfinished herd cases.**
  [herd.feature](tests/features/herd.feature),
  [forall.feature](tests/features/forall.feature), and
  [reset.feature](tests/features/reset.feature) request parallel tests.
  Herd also has disabled cases for ref state and tags. Prioritize equivalent
  results for one and multiple jobs, failure propagation, dirty-worktree checks,
  detached HEAD, and annotated/lightweight tags using controlled local fixtures.

## Maintain documentation and release tooling

- [ ] **Repair the documentation build and API pages.**
  [script/docs](script/docs) reads `$1` with `set -u`, so a no-argument call can
  fail after the build. Use an optional argument safely and verify both build and
  preview modes. Regenerate API stubs and remove obsolete references to
  `clowder.data`, `clowder.git_project`, and former top-level controller modules
  in [docs/rst](docs/rst/). Build the docs and resolve stale import/link warnings.

- [ ] **Update contributor and installation guidance.**
  [CONTRIBUTING.md](CONTRIBUTING.md) directs users through `script/update` and
  a package-directory `PYTHONPATH`; [README.md](README.md) uses `sudo pip`.
  After the setup baseline is verified, document an isolated editable install,
  interpreter selection, test markers and side effects, and the current
  INI-style config. Check examples against CLI help and the schema; clarify
  historical docs such as `clowder-yml-syntax-reference-old.md`.

- [ ] **Restore release preconditions and package checks.**
  [script/deploy](script/deploy) has disabled branch/clean-tree assertions and
  a TODO to load release notes. Restore appropriate checks and separate package
  validation from publication. Build a wheel and source archive; verify the CLI
  entry point, bundled schema, and metadata in a fresh install. Use
  [CHANGELOG.md](CHANGELOG.md) for release notes, and test failure paths without
  creating releases or uploading packages.
