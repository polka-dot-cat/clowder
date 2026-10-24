# Clowder repository transfer plan

Implementation started on October 23, 2026. See the
[progress record](clowder-repository-transfer-status.md) for completed work,
validation results, and the deleted test-fork assessment. The owner transferred
the repository and merged PR #641 on October 24; the planned manifest cutover
is complete. The remaining work is CI cleanup and hosted validation.

## Recommended approach

Move `JrGoodle/clowder` to `polka-dot-cat/clowder` through GitHub’s native repository transfer. Prepare the migration changes first, then merge them immediately after the transfer. Complete this move before starting the existing TODO backlog.

Keep the repository name `clowder`, public visibility, default branch `master`, and Python package name `clowder-repo` during the move. Retain the existing `clowder.cat` and `gh-pages` configuration for later website work. The owner has deferred DNS updates, Pages verification, and website restoration; website availability does not block the repository transfer.

A native transfer preserves Git history, issues, pull requests, stars, watchers, and fork relationships. GitHub redirects the old repository URL and Git clone, fetch, and push operations. [GitHub transfer documentation](https://docs.github.com/en/repositories/creating-and-managing-repositories/transferring-a-repository)

## Repository findings

The following state was checked on October 15, 2026.

- The source repository is public, is not a fork, and uses `master` as its default branch.
- The authenticated account has repository admin access and is an active owner of `polka-dot-cat`. No repository named `clowder` appeared in the organization listing.
- GitHub Pages serves `https://clowder.cat/` from the root of the `gh-pages` branch.
- A local scan found old GitHub-owner references in 296 files, including 279 under `tests/`. This is an inventory of references, not a count of files that all require replacement.
- The transferred [cats manifest](https://github.com/polka-dot-cat/cats/blob/master/clowder.yml) still names projects as `JrGoodle/mu`, `JrGoodle/duke`, and other personal-account paths. Clowder tests assert project names as well as clone URLs.
- Fixtures also reference `felidae`, `djinni`, `gyp`, and `sox`, which were absent from the returned organization listing. Confirm their destinations or historical status before changing references.
- The current CLI token cannot read the organization’s Actions policy. Check that policy in GitHub settings before transfer.
- The local Python 3.8.16 interpreter and project Pipenv environment were missing during the initial repository review. Test execution needs an appropriate environment.

## Transfer steps

### 1. Record the current state

Create a Git mirror and record the repository ID, branches, tags, releases, branch protections, collaborators, Pages settings, and installed integrations. Inventory secret names and credential dependencies without copying secret values into this plan. Record existing CI failures so they remain distinct from migration failures. A Git mirror backs up Git data; retain GitHub settings and release-asset information separately.

### 2. Prepare the repository and fixture changes

Create an explicit mapping from each old repository name to its intended destination. Prepare a focused migration PR and any matching changes needed in the transferred test repositories.

| Area | Changes to prepare |
| --- | --- |
| Project links | Repository URL in `setup.py`; README, contributor guide, documentation, and badges |
| Scripts | `script/process_readme.py`, `script/website`, and CircleCI artifact URLs |
| Test fixtures | Repository URLs, manifest names, expected YAML, and `PROJECT_NAME` assertions |
| Transferred repositories | Manifest contents and applicable `.gitmodules` URLs |

Update names and their expected output together. Preserve author credits, local filesystem paths, and historical commit references. Edit `README.md` and its processing script; regenerate `README-processed.md` through the existing build process.

Coordinate updates to mutable test branches with the Clowder fixture changes. Keep tags and commit IDs stable so historical fixtures remain reproducible.

Keep the published Cats and clowder-examples manifest names compatible until
PR #641 merges. The renamed manifests are staged on `org-migration/master` and,
for Cats, `org-migration/alt-branch`. Validate the draft against these branches;
publish them to the active branches only at the coordinated merge. See the
[cutover record](clowder-repository-transfer-status.md#cats-and-felidae-migration)
for the exact commits. Retain the separate Beanstalk dependency.

### 3. Prepare retained service access

Website work is deferred to a separate
[TODO](https://github.com/polka-dot-cat/clowder/blob/master/TODO.md#deferred-website-work--after-transfer).
The [preflight record](clowder-transfer-preflight.md#pages-and-dns) preserves the
DNS, HTTPS, Pages source, and domain-verification findings for that follow-up.
Keep the existing configuration during transfer. The site may be unavailable
until the deferred work is complete.

Check the organization’s Actions policy, allowed third-party actions, workflow token permissions, and access to test repositories. Organization policy can restrict which repositories and actions may run. [GitHub Actions organization settings](https://docs.github.com/en/organizations/managing-organization-settings/disabling-or-limiting-github-actions-for-your-organization)

Identify which configured services remain active: CircleCI, Read the Docs, Codecov, and Code Climate. Prepare any required organization authorization or project reconnection. Repository secrets, webhooks, and deploy keys stay associated with a transferred repository; verify external service access separately. [GitHub transfer behavior](https://docs.github.com/en/repositories/creating-and-managing-repositories/transferring-a-repository)

The [service preflight record](clowder-transfer-preflight.md) now contains the
owner-confirmed Actions/Pages settings, deferred DNS changes, CI repair, and service
failures that need account-side work. Requires.io and Read the Docs were disabled
with the owner's approval. Qlty and Codecov are also deferred: the legacy Code
Climate webhook is disabled, coverage uploads remain off, and separate
[TODOs](https://github.com/polka-dot-cat/clowder/blob/master/TODO.md#deferred-reporting-integrations--after-transfer) cover later
reactivation. They are not transfer prerequisites.
CircleCI is retained; account access, checkout, and hosted write tests are
restored. After the owner registered the separate additional SSH key with
GitHub, [job 2982](https://circleci.com/gh/polka-dot-cat/clowder/2982) passed all 89
write cases with no failures or skips. The required GitHub status is successful.
The owner confirmed CircleCI's GitHub OAuth access to `polka-dot-cat` is granted.
After transfer, verify the project's configuration, keys, and required status.
The preflight record contains the hosted evidence and key recovery history.
Keep the remaining service checks distinct from successful GitHub webhook delivery.

### 4. Transfer ownership

In the existing repository, open **Settings → General → Danger Zone → Transfer**. Select `polka-dot-cat`, retain the name `clowder`, review GitHub’s transfer details, and complete the transfer.

The ownership transfer can proceed while PR checks are running. The owner elected to merge PR #641 before its latest checks completed;
that validation remains explicitly pending. Let any old-path CircleCI write jobs finish before
triggering jobs at the new path: the serial group uses `pipeline.project.slug`,
so jobs using the two paths could otherwise mutate the shared fixtures at once.
Recheck CI after transfer and rerun interrupted checks as needed.

Update each local clone’s remote:

```sh
git remote set-url origin git@github.com:polka-dot-cat/clowder.git
```

Leave `JrGoodle/clowder` unused. Creating a replacement repository at that location can permanently remove the redirect. GitHub Pages URLs do not receive the repository-transfer redirects. [GitHub transfer instructions](https://docs.github.com/en/repositories/creating-and-managing-repositories/transferring-a-repository)

### 5. Merge and verify

Merge the prepared migration changes and coordinate the test-repository updates. Confirm access, refs, releases, issues, protections, and clone behavior at the new location. Check the old repository redirect.

Website restoration, HTTPS checks, and Pages deployment validation are deferred
to the website TODO. Read the Docs is disabled for now at the owner's request.
If reactivated later, update the existing project's
repository URL, configure the webhook secret, and verify its build trigger while
retaining its documentation address. The repository URL is configured in the
project's settings. [Read the Docs repository setting](https://docs.readthedocs.com/platform/latest/reference/environment-variables.html)

Run focused tests for changed names, URLs, and expected output. Verify SSH access separately, and run tests that change remote branches only against the intended test repositories. Keep existing Python and CI defects separate from the migration changes.

### 6. Keep package publishing independent

Retain `pip install clowder-repo`. Update package source links for the next release. A repository transfer does not require a package rename or an immediate release. Moving the package into a PyPI organization is a separate ownership operation. [PyPI project transfers](https://docs.pypi.org/organization-accounts/actions/project-actions/)

## Tradeoffs

| Choice | Benefit | Cost |
| --- | --- | --- |
| Native transfer — recommended | Preserves the GitHub project and its continuity | Requires checks of organization policies and integrations |
| New repository with a Git mirror | Provides fresh repository settings | Git alone does not carry issues, PRs, stars, or release assets |
| Prepare changes first, merge after transfer — recommended | Shortens the transition and keeps the changes reviewable | Requires coordination with test-repository manifests |
| Transfer first and rely on redirects | Moves ownership quickly | Leaves stale project names, badges, and service connections |
| Combine transfer with TODO repairs | Groups maintenance into one effort | Delays the move and makes failures harder to attribute |

The organization provides shared ownership and a common home for Clowder and its test repositories. It also introduces organization-level access and Actions policies that must be maintained.

If validation fails, restore the affected configuration or revert the migration commits as appropriate. Keep the saved settings and Git refs available. Assess a reverse ownership transfer separately; it does not undo changes to external services, DNS, or fixture contents.

## Completion checks

- [x] `polka-dot-cat/clowder` is the intended public repository, with the expected admins and collaborators.
- [x] Branches, tags, releases, issues, and pull requests match the pre-transfer inventory.
- [x] Branch protections and required checks are reviewed under the organization.
- [x] Old repository links redirect, and local clones use the new remote.
- [x] Website DNS, Pages, and domain-verification work is explicitly deferred to its own TODO; website availability does not block transfer completion.
- [x] Read the Docs is explicitly deferred; its GitHub webhook is disabled and build badges are removed.
- [x] Qlty and Codecov are explicitly deferred, with uploads inactive, the legacy Code Climate webhook disabled, badges removed, and separate follow-up TODOs.
- [ ] Retained CI and reporting services can access the repository.
- [x] Active test manifests and Clowder assertions use the agreed repository mapping.
- [ ] Focused migration tests pass; any unrelated build or environment failures are recorded separately.
- [x] Package naming remains `clowder-repo`, with corrected source links prepared for the next release.

Resume the existing TODO backlog after the transfer checks are complete.
