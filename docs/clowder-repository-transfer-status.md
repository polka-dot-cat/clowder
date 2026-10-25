# Repository transfer progress

Updated October 24, 2026 (America/Chicago). Follow the
[transfer plan](clowder-repository-transfer-plan.md) for the full sequence.

## Current state

The owner transferred the repository to
[polka-dot-cat/clowder](https://github.com/polka-dot-cat/clowder) on October 24
and merged [PR #641](https://github.com/polka-dot-cat/clowder/pull/641) at
`ecfdc14ccd91c4f310fe079476185f7f17c5db13`. The owner chose to merge before the
latest checks completed. Earlier successful baselines remain recorded below;
they do not establish success for the merged commit.

Repository ID `38194118`, public visibility, `master`, collaborator permissions,
all five required checks, secret names, environments, 40 tags, 40 releases, and
all 78 release-asset identities and sizes were preserved. The immediate transfer
inventory found all 14 expected branches; only the already-published migration
branch had advanced since the earlier snapshot. Open issue/PR numbers and issue
assignees were preserved. The local `origin` now uses the organization URL.

The old GitHub API URL resolves to the same repository. CircleCI retains project
ID `bb8973a7-cc5b-4870-affe-eb39676286fb` at `gh/polka-dot-cat/clowder` and the
read-only checkout key. Requires.io, Read the Docs, and Code Climate webhooks
remain disabled; CircleCI and Slack remain active.

After merge, Cats and clowder-examples were fast-forwarded to the prepared
manifest commits. This follow-up removes the temporary CI fixture overrides and
the unused Codecov orb that blocked the first organization pipeline. Hosted CI
validation after this cleanup remains pending. See the
[integration record](clowder-transfer-preflight.md#post-transfer-checks).

`felidae` has been transferred to
[polka-dot-cat/felidae](https://github.com/polka-dot-cat/felidae). Its repository ID
is still `294826031`. At transfer, both branches and both tags matched the backup,
and the old URL resolved to the new owner. Subsequent submodule commits are
recorded below. The original mirror and its LFS object are saved.

The replacement test repositories are now available under `polka-dot-cat`:
[djinni](https://github.com/polka-dot-cat/djinni),
[gyp](https://github.com/polka-dot-cat/gyp), and
[sox](https://github.com/polka-dot-cat/sox). Their default branch is `master`.
Their published manifests and the matching local Clowder assertions now use the
organization names. All 97 cases affected by the deleted forks and one new
saved-defaults regression pass. The remaining Cats and clowder-examples manifest
changes are now published on their active branches after PR #641 merged.

## Backups and inventory

The local backup directory, relative to the Clowder checkout, is
`../clowder-transfer-backups/20261024T023608Z/`. The UTC timestamp falls on
October 23 in the workstation's timezone. `../clowder-transfer-backups/latest.txt`
contains its absolute path. Keep these backups outside the repository.

- `clowder.git`: a mirror of the remote Git repository; `git fsck --full` passed.
- Git LFS fetch completed for all 149 Clowder objects.
- GitHub inventory: 13 branches, 40 tags, 40 releases, 55 open issues/PRs combined,
  including four open PRs; collaborators, branch protection, Pages, Actions,
  environments, and integration metadata were recorded.
- All 78 release assets were downloaded; sizes were checked and SHA-256 hashes
  recorded in `release-assets.json`.
- Secret names were inventoried, without secret values. Webhook URLs are recorded
  only as service hosts, without embedded credentials.
- `felidae.git` and `felidae-inventory.json` retain the test repository's
  pre-transfer state.
- Test collection, dependency analysis, and validation logs are saved beside the
  backups. The GitHub inventory is a configuration snapshot, not a complete
  export of issue comments or other GitHub application data.
- The October 24 preflight refresh adds a verified Git bundle through commit
  `6607ba77` and a fresh settings/ref inventory, without replacing the original
  backups. All 78 release assets still match their recorded sizes and hashes.
  See the [preflight evidence paths](clowder-transfer-preflight.md#evidence).

## Repository mapping

| Existing reference | Confirmed destination or status |
| --- | --- |
| `JrGoodle/clowder` | Transferred to `polka-dot-cat/clowder`; repository ID preserved |
| `JrGoodle/cats`, `clowder-examples`, `misc-clowder-tests`, `swift-clowder` | Same repository names under `polka-dot-cat`; redirects verified |
| `JrGoodle/mu`, `duke`, `kit`, `kishka`, `sasha`, `june`, `duffy` | Same repository names under `polka-dot-cat`; redirects verified |
| `JrGoodle/felidae` | Transferred to `polka-dot-cat/felidae` in this transition |
| `JrGoodle/djinni`, `gyp`, `sox` | Recreated under `polka-dot-cat`; published manifests and local test assertions now use the organization URLs |
| `JrGoodle/ash` | Transferred to `polka-dot-cat/ash`; active submodule URLs updated |
| `JrGoodle/swift` | Existing personal fork; no destination change selected |

The merged patch updates project links, build metadata, website and artifact
scripts, test-repository clone URLs, initialization expectations, and manifest
project names. Published Cats manifests now match those assertions.

The YAML source-URL tests also use a distinct Beanstalk host,
`polka-dot-cat.git.beanstalkapp.com`. Those references are not GitHub organization
URLs. The owner chose to retain Beanstalk; its access check remains pending.

## Deleted fork impact

The initial collection found **97 affected test cases out of 1,110**, across 16 scenario
definitions. Counts include expanded outline rows and fixture parameters, not
just files containing a repository name.

| Coverage | Cases | Purpose |
| --- | ---: | --- |
| `config.feature` | 12 | HTTPS/SSH protocol configuration and origin/upstream URLs |
| `herd.feature` | 25 | Default and overridden protocols; remote URLs; branch behind upstream |
| `forall.feature` | 3 | Project/upstream environment variables and script failure handling |
| YAML upstream validation | 57 | Explicit/inherited remote names and source protocols |
| **Total** | **97** | 40 command cases plus 57 YAML-validation cases |

By repository, the overlapping counts are `djinni`: 69, `gyp`: 68, and `sox`: 40.
All 40 miscellaneous command cases initialize or herd the same three-project
manifest, so they depend on all three forks even when a row asserts only one
project. Of the 97 cases, 75 carry the `ssh` marker and 22 do not.

The count came from pytest collection plus each case's selected YAML fixture
and miscellaneous manifest. Before the changes, one HTTPS herd case reproduced
Git's "Repository not found" error while cloning `JrGoodle/djinni`. After the
manifest and assertion updates, all 97 cases were executed and passed. Legacy
shell tests and documentation examples are excluded from this count. Normal
application use does not require these repositories.

### Selected replacement strategy

**Completed: recreated organization-owned copies from the original upstream
histories.** This preserves the existing test goals and limits the amount of
fixture redesign during the ownership move.

| Repository | Source and required state |
| --- | --- |
| `polka-dot-cat/djinni` | Native GitHub fork of [dropbox/djinni](https://github.com/dropbox/djinni). Both source branches match; no tags were advertised. The archived upstream remains separate from the writable fork. |
| `polka-dot-cat/gyp` | Imported all three branches from [Chromium's GYP repository](https://chromium.googlesource.com/external/gyp/); no tags were advertised. Created `fork-branch` at [bd11dd1c51ef17592384df927c47023071639f96](https://github.com/polka-dot-cat/gyp/commit/bd11dd1c51ef17592384df927c47023071639f96). |
| `polka-dot-cat/sox` | Imported both branches and all 31 tags from [the SourceForge SoX Git repository](https://sourceforge.net/p/sox/code/ci/master/tree/). The SourceForge HTTPS upstream remains separate. |

GYP and SoX are Git imports into new GitHub repositories, since their sources are
hosted outside GitHub; they do not have GitHub fork-network relationships. All
imported branch and tag object IDs match their source snapshots. The new GYP
branch is 23 commits behind upstream `master` at verification time.

A fresh clone of GYP's `fork-branch` successfully ran
`git pull --ff-only upstream master`, reached
`1615ec326858f8c2bd8f30b3a86ea71830409ce4`, and remained on `fork-branch` with a clean
working tree. This check changed only the local verification clone. The GitHub
branch still points to `bd11dd1c51ef17592384df927c47023071639f96`.

Source clones, push logs, ref comparisons, and the fast-forward check are saved
under `fixture-restoration-71mfcsze/` in the backup directory above. The new
repository IDs are `1431752815` (Djinni), `1431753424` (GYP), and `1431753437` (SoX).

### Published manifest changes

- [misc-clowder-tests: `58472a5`](https://github.com/polka-dot-cat/misc-clowder-tests/commit/58472a5ae97ef5a4a1a33622132341439c4c028c)
  updates the root `clowder.yaml` and the `defaults` and `https` saved versions.
- [clowder-examples: `0a4714b`](https://github.com/polka-dot-cat/clowder-examples/commit/0a4714bdecbfb1fb9175cf870b490d607b964ddb)
  updates the three restored-fork names in `versions/upstreams-2.clowder.yml`.
- [misc-clowder-tests: `431c87e`](https://github.com/polka-dot-cat/misc-clowder-tests/commit/431c87ebcc8dfcef35007d4cab724ea973377f51)
  explicitly selects Chromium for GYP's upstream in the saved `defaults` version.

These commits were pushed to `master` and verified against GitHub's advertised
refs. The local Clowder changes update 45 YAML fixtures, four feature files, the
upstream environment-variable script, and the upstream example guide. The test
README now records the dependencies, GYP branch requirement, and commands to run
the 98 cases, including the new regression. These Clowder changes are included in
the local migration checkpoint on `codex-cleanup`.

Validation first used the prepared `misc-clowder-tests` commit through a Git URL
override scoped to the test process. Clones of Djinni, GYP, SoX, and their
upstreams used their actual hosting services. Each `misc-clowder-tests` commit
was tested before publication. The pushes preserved history and existing tags.

A fresh-clone check exposed an existing error in the saved `defaults` version:
GYP's upstream fell back to the GitHub source, so it tried to fetch
`https://github.com/external/gyp.git`. The project-level source default does not
set the upstream's source. Commit `431c87e` makes that source explicit. The new
`herd saved defaults with explicit upstream sources` scenario verifies all three
origin/upstream URL pairs and GYP's starting branch and commit.

The working clones, original local files, test logs, and publication records are
saved under `fixture-migration-acvglxfc/` in the backup directory above. The
three restored test repositories still have the same branch and tag IDs as their
post-creation inventory; GYP's remote `fork-branch` remains at the required commit.

**Alternative: small purpose-built Git fixtures.** Use a shared ancestor, an
upstream branch with later commits, and a fork branch behind it. This provides
controlled remote names, protocols, and branch state with less data and fewer
external dependencies. Local bare repositories can cover most behavior; retain
small network integration tests for GitHub SSH/HTTPS and mixed hosting services.
This is the better long-term test design, but requires fixture and assertion
changes beyond restoring the existing examples. Replacing all three with random
public repositories would retain the availability risk and could lose coverage
of mixed hosts or branch history.

## Cats and Felidae migration

The owner chose to retain Beanstalk and keep published Cats manifest names
compatible until PR #641 merged. The merged patch updates the matching project names,
CLI arguments, expected YAML, environment-variable checks, and Cats guide.
Author credits, local filesystem paths, historical recordings, and fixed refs
remain unchanged.

### Published manifest cutover

After PR #641 merged, the active branches were fast-forwarded to these prepared
commits. The Cats updates used one atomic push. All unrelated refs were preserved:

| Repository / active branch | Previous active commit | Migration branch | Published commit |
| --- | --- | --- | --- |
| `cats` / `master` | `2202007cc3d1cb957c7f4bfc08797b3350125e33` | `org-migration/master` | [`6767bc8`](https://github.com/polka-dot-cat/cats/commit/6767bc8cb483775cb13ecdf9ac50bb24307f2130) |
| `cats` / `alt-branch` | `664d3c130482b941cfe96b5672d9ca49f1dc397e` | `org-migration/alt-branch` | [`ee06522`](https://github.com/polka-dot-cat/cats/commit/ee065226e79f8001478cae930bbff1a1564befcd) |
| `clowder-examples` / `master` | `0a4714bdecbfb1fb9175cf870b490d607b964ddb` | `org-migration/master` | [`c1dd3c2`](https://github.com/polka-dot-cat/clowder-examples/commit/c1dd3c286dcafdfe4a28384aebe60f748fe1d168) |

Active and migration refs were checked immediately before publication, and each
update was confirmed as a fast-forward. The resulting advertised refs match the
planned state exactly. Migration branches remain available as historical markers.
Use the [published-fixture instructions](../tests/README.md#testing-the-published-manifests)
for tests; no local Cats URL override is needed.

### Published submodule changes

`ash` was transferred to [polka-dot-cat/ash](https://github.com/polka-dot-cat/ash).
Repository ID `104600176`, both branches, and public visibility were preserved
at transfer. It had no tags, releases, or Pages site. Its Git backup passed
`git fsck --full`.

These compatible URL and gitlink updates are now published:

| Repository / branches | Published commit | Change |
| --- | --- | --- |
| `ash`: `master`, `herd-branch` | [`07b3ff8`](https://github.com/polka-dot-cat/ash/commit/07b3ff8280a5a67efaa96d324690e39d2de7b61d) | Use the canonical `polka-dot-cat/duffy` URL; preserve the Duffy gitlink. |
| `mu`: `submodules` | [`eb6f03f`](https://github.com/polka-dot-cat/mu/commit/eb6f03ffe54f7b87bd14f7e38baef3c0d3b54aba) | Use `polka-dot-cat/ash` and its updated commit. |
| `felidae`: `master`, `felid` | [`7c6f96c`](https://github.com/polka-dot-cat/felidae/commit/7c6f96c3eecbb693a75a79213c8dade99da380ba) | Use `polka-dot-cat/ash` and its updated commit. |

Recursive clone checks passed for all three parent branch configurations before
publication. All unrelated advertised refs, including tags, were unchanged
after publication. Historical tags and the fixed Felidae commit retain their
original objects; old submodule URLs in that history rely on GitHub redirects.
The `felid` branch and tag are distinct refs and must not be confused.

A fresh clone of the published Cats `master`, with its original `JrGoodle/...`
names, successfully linked the `submodules` version and herded all projects.
Mu, Ash, and Duffy were clean at the expected commits, and both nested
`.gitmodules` URLs were canonical. This check used the live GitHub repositories
without a URL override.

### Beanstalk access: pending

Retain `polka-dot-cat.git.beanstalkapp.com`, its `beanstalk` source alias, and the
`polka-dot-cat/felidae` path. There are **28 explicit source-URL test cases** that
need this separate service. Their YAML still passes the same schema checks.

The read-only `git ls-remote` attempt stopped at host-key verification before
account authentication. SSH is configured to use 1Password; the owner confirmed
that the intended personal public key is already added to Beanstalk. This does
not establish whether authentication or repository access succeeds.

The observed RSA server fingerprint was
`SHA256:pmFiLpVdAFqGS5dsrE1Wrk5U/wKoPJKg+M7+grPQwNw`. It is **unverified**:
the connection itself is not an independent source of trust. The owner chose
to leave validation pending. Verify it through a trusted existing connection
or Beanstalk support, then configure host trust and retry:

```sh
GIT_SSH_COMMAND='ssh -oBatchMode=yes -oConnectTimeout=10 -oStrictHostKeyChecking=yes' \
  git ls-remote git@polka-dot-cat.git.beanstalkapp.com:polka-dot-cat/felidae.git
pipenv run python -m pytest tests/functional/test_schema_validation.py -q \
  -k 'source_url and not implicit'
```

No Beanstalk key, account setting, repository, or local known-host entry was
changed.

The fixtures, before/after inventories, candidate commits, publication records,
and validation logs for this phase are saved under
`cats-felidae-migration-t3_1y2_0/` in the backup directory above.

## Validation

Python 3.8.16 and Pipenv were unavailable. Validation used the installed Python
3.10.11, within the declared supported range, in an isolated environment outside
the checkout. Dependencies came from `requirements.txt`; the environment also
used `setuptools==67.8.0` and `wheel==0.40.0` for editable installation. Repository
interpreter and dependency pins were unchanged.

- Editable installation and `python -m clowder.app --help`: passed.
- Empty-directory help selection: **18 passed**.
- HTTPS initialization selection, excluding SSH/offline/write/help/subdirectory
  cases: **10 passed**.
- Test collection: **1,110 cases**, without executing fixtures.
- Before restoration: **one expected failure**, at the missing `djinni` clone.
- After manifest migration: **97 passed**, covering all 40 miscellaneous command
  cases and 57 affected upstream YAML cases, including HTTPS and SSH. The initial
  smoke case passed in 8.78 seconds; the other 96 passed in 576.46 seconds.
- Saved-defaults source regression: **one additional case passed** in 9.27 seconds,
  bringing the relevant selection to **98 passing cases**.
- Fresh-clone verification of published commit `431c87e`: the saved `defaults`
  manifest herded all three projects successfully with the expected origin and
  upstream URLs, clean working trees, and the required GYP branch and commit.
- All 49 changed YAML manifests across the three repositories passed JSON Schema
  validation; the changed environment-variable script passed `bash -n`.
- Changed Python files parse, changed shell scripts pass `bash -n`, local
  documentation links resolve, and `git diff --check` passes.
- Sphinx HTML build: the latest incremental build completed with **41 warnings**;
  the earlier full build had 42. These include stale API imports, existing
  configuration problems, and pages outside the toctree (including these two
  transfer documents). Documentation cleanup remains in the existing backlog.

For the Cats/Felidae phase, all 220 changed local YAML files parse and retain
their prior schema-validation results, including intentionally invalid cases.
All 31 staged external manifests pass JSON Schema validation. Both changed
environment-variable scripts pass `bash -n`. **473 selected cases passed**: 319
Cats command cases and 154 GitHub-only project YAML integration cases. This
includes eight initial smoke cases. The remaining cases ran in audited batches
with isolated temporary workspaces; no completed case was repeated. The 28
explicit Beanstalk source-URL cases remain pending. Collection now finds 1,111
cases, including the earlier saved-defaults regression.

A final comparison of branches and tags across all 11 audited repositories
matched the planned state. GYP's remote `fork-branch` is also unchanged at its
required commit.

This fixture-validation phase did not run tests that disable host networking,
modify remote branches, or uninstall global/system LFS configuration. The later
[preflight phase](clowder-transfer-preflight.md#github-actions) ran the three LFS
cases in a disposable container. These checks do not establish a full CI
baseline. The initial inventory had no Actions/check runs. The initial draft PR had
[Actions run 40058163818](https://github.com/JrGoodle/clowder/actions/runs/40058163818),
which failed before tests because the workflow used deprecated
`actions/upload-artifact@v2`; the other matrix jobs were cancelled. The subsequent
[preflight repair](clowder-transfer-preflight.md#github-actions) updates the
workflow and preserves the existing required-check names. At `6607ba77`, all
four Actions jobs pass all 684 cases, including LFS, and the Ubuntu/Python 3.11
documentation build passes. Reports are saved in the preflight evidence directory.
CircleCI's required
`test_write` check now loads its configuration and checks out the repository with
the restored deploy key. Its active test job was repaired after the legacy
Code Climate reporter returned HTML. All 89 write cases passed against local
bare fixture copies in an isolated
Python 3.11 container with networking disabled. The owner subsequently registered
CircleCI's separate additional public key with GitHub. Hosted
[job 2982](https://circleci.com/gh/polka-dot-cat/clowder/2982), at `e94b8e33`, then
passed all **89 write cases**, with no failures, errors, or skips, confirmed from
its JUnit artifact. Its required GitHub status is successful. The job completed
in 41 minutes, including 39m 43s of tests against the real GitHub SSH fixtures.
The working checkout key remains read-only. See the
[hosted validation record](clowder-transfer-preflight.md#hosted-circleci-write-validation).

After publication, five focused tests passed against the actual GitHub URLs
without fixture overrides: raw/resolved YAML, project environment variables,
and recursive submodules. The run used the existing Python 3.10 validation
environment with its `bin` directory on `PATH` for the harness's `python` child
processes. The first attempt stopped before Clowder ran because that PATH entry
was missing; correcting the test environment resolved it.

## Remaining transfer steps

1. Merge the CI cleanup that removes temporary fixture overrides and the unused
   Codecov orb. Confirm hosted Actions and CircleCI checks against published
   manifests under the organization. The first post-transfer CircleCI pipeline
   stopped during configuration, before running tests.
2. Complete the remaining [verification checks](clowder-repository-transfer-plan.md#completion-checks).
   Beanstalk's 28 source-URL cases remain pending at the owner's request.
   Qlty and Codecov remain inactive, with separate follow-up TODOs. Requires.io
   and Read the Docs are disabled; Slack remains configured.

The owner has deferred `clowder.cat` DNS changes, Pages/domain verification,
and website restoration to a separate
[TODO](https://github.com/polka-dot-cat/clowder/blob/master/TODO.md#deferred-website-work--after-transfer).
Retain the current configuration during the move. Website availability is not
a transfer prerequisite; the site can be restored later.
