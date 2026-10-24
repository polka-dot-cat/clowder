# Examples

For the native CLI, first run `cargo build --locked`, then set
`CLOWDER_EXECUTABLE="$PWD/target/debug/clowder"` when invoking pytest from the
repository root. Without that variable the harness uses the Python reference
with the same interpreter as pytest. `cargo test --locked` runs local native
regressions without contacting the shared fixtures.

The GitHub fixtures and Clowder assertions use `polka-dot-cat/...` project
names. PR #641 is merged, and the Cats/clowder-examples manifest cutover is
complete. See the [transfer record](../docs/clowder-repository-transfer-status.md).

## Cats

https://github.com/polka-dot-cat/cats.git

| Directory | Repository |
|-----------|------------|
| `black-cats/june` | https://github.com/polka-dot-cat/june |
| `black-cats/kishka` | https://github.com/polka-dot-cat/kishka |
| `black-cats/kit` | https://github.com/polka-dot-cat/kit |
| `black-cats/sasha` | https://github.com/polka-dot-cat/sasha |
| `duke` | https://github.com/polka-dot-cat/duke |
| `mu` | https://github.com/polka-dot-cat/mu |

### Testing the published manifests

Use the published Cats `master` and `alt-branch` directly. The temporary Git URL
overrides and pinned migration commits are no longer needed.

```sh
pipenv run python -m pytest tests/functional/test_features.py -q \
  -m 'cats and not (ssh or offline or write or lfs or help or init)' \
  -k 'forall or yaml or submodules'
```

GitHub Actions runs 14 initialization cases, 667 functional cases, and three
serial LFS cases per matrix job against published URLs. CircleCI runs the 89
`write and not offline` cases through GitHub SSH, with coverage, JUnit, and
Cucumber artifacts. It selects its additional SSH key explicitly and checks
write access to all six child repositories with dry-run pushes. Its checkout
key remains read-only. See the
[hosted validation record](../docs/clowder-transfer-preflight.md#hosted-circleci-write-validation).

The SSH fixtures delete remote `pytest*` branches during setup, and write cases
create and delete more temporary branches. CircleCI serializes these jobs; do
not run another write suite against the shared fixtures concurrently. A canceled
job can leave temporary branches, which the next SSH fixture setup removes.

### Felidae, submodules, and Beanstalk

The GitHub YAML fixtures use `polka-dot-cat/felidae`. Its active `master` and
`felid` branches, and `mu`'s `submodules` branch, now reference
`polka-dot-cat/ash`; Ash references `polka-dot-cat/duffy`. Tags and fixed commits
retain their original history and can still contain URLs served through GitHub
redirects.

The explicit `source.url` scenarios retain
`git@polka-dot-cat.git.beanstalkapp.com:polka-dot-cat/felidae.git`.
This is a separate hosting service, even though the project name matches GitHub.
The 28 explicit source-URL cases need Beanstalk SSH access and a verified server
host key. Adding an account public key does not establish server trust.

The GitHub-only project YAML cases can run separately:

```sh
pipenv run python -m pytest tests/functional/test_schema_validation.py -q \
  -k 'project and not upstream and not git_lfs and (not source_url or implicit)'
```

These YAML cases use GitHub SSH but do not use the Cats setup that deletes
remote `pytest*` branches. Exclude the `lfs` scenarios on a shared workstation:
some of their steps modify global/system Git LFS settings.

## Misc

https://github.com/polka-dot-cat/misc-clowder-tests.git

The root manifest and its `defaults` and `https` saved versions use these
repositories. The integration scenarios need both each fork and its upstream.

| Directory | Fork | Upstream | Branch |
| --- | --- | --- | --- |
| `djinni` | [polka-dot-cat/djinni](https://github.com/polka-dot-cat/djinni) | [dropbox/djinni](https://github.com/dropbox/djinni) | `master` |
| `gyp` | [polka-dot-cat/gyp](https://github.com/polka-dot-cat/gyp) | [Chromium GYP](https://chromium.googlesource.com/external/gyp/) | `fork-branch` |
| `sox` | [polka-dot-cat/sox](https://github.com/polka-dot-cat/sox) | [SourceForge SoX](https://sourceforge.net/p/sox/code/ci/master/tree/) | `master` |

Keep GYP's remote `fork-branch` at
`bd11dd1c51ef17592384df927c47023071639f96`. The upstream-update scenario starts
there and advances only its temporary local branch. Updating the GitHub branch
would remove the starting state that the test checks.

Run the 41 miscellaneous command cases and 57 related upstream YAML cases with:

```sh
pipenv run python -m pytest tests/functional/test_features.py -q -m misc
pipenv run python -m pytest tests/functional/test_schema_validation.py -q -k 'upstream and (not source_url or implicit)'
```

These selections require network and GitHub SSH access. They use temporary local
repositories; the selected setup paths do not delete or push remote branches.
Recheck that assumption if the fixtures change.

## Swift

https://github.com/polka-dot-cat/swift-clowder.git

| Directory | Repository |
|-----------|------------|
| `clang` | https://github.com/apple/swift-clang |
| `cmark` | https://github.com/apple/swift-cmark |
| `compiler-rt` | https://github.com/apple/swift-compiler-rt |
| `llbuild` | https://github.com/apple/swift-llbuild |
| `lldb` | https://github.com/apple/swift-lldb |
| `llvm` | https://github.com/apple/swift-llvm |
| `ninja` | https://github.com/ninja-build/ninja.git |
| `swift` | https://github.com/apple/swift |
| `swift-corelibs-foundation` | https://github.com/apple/swift-corelibs-foundation |
| `swift-corelibs-libdispatch` | https://github.com/apple/swift-corelibs-libdispatch |
| `swift-corelibs-xctest` | https://github.com/apple/swift-corelibs-xctest |
| `swift-integration-tests` | https://github.com/apple/swift-integration-tests |
| `swift-xcode-playground-support` | https://github.com/apple/swift-xcode-playground-support |
| `swiftpm` | https://github.com/apple/swift-package-manager |
