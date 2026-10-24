# ![clowder](docs/resources/clowder.png)

> **clowder** - A group of cats
>
> **herding cats** - An idiom that refers to a frustrating attempt to control or organize a class of entities which are uncontrollable or chaotic

This branch contains the Rust migration in progress. Build the native CLI with
`cargo build --locked` and run `target/debug/clowder`. See the
[migration guide](docs/rust-migration.md) for requirements and validation.
The Python installation instructions below describe the existing published release.

|  |  |
|:-|:-|
| docs | [Documentation](https://clowder.cat) |
| tests | [![GitHub Actions Build Status](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Factions-badge.atrox.dev%2Fpolka-dot-cat%2Fclowder%2Fbadge&style=flat)](https://actions-badge.atrox.dev/polka-dot-cat/clowder/goto) [![CircleCI](https://circleci.com/gh/polka-dot-cat/clowder.svg?style=shield)](https://circleci.com/gh/polka-dot-cat/clowder) |
| package | [![PyPI version](https://badge.fury.io/py/clowder-repo.svg)](https://badge.fury.io/py/clowder-repo) [![Python versions](https://img.shields.io/pypi/pyversions/clowder-repo.svg)](https://pypi.python.org/pypi/clowder-repo) [![License](https://img.shields.io/pypi/l/clowder-repo.svg)](https://pypi.python.org/pypi/clowder-repo) [![Status](https://img.shields.io/pypi/status/clowder-repo.svg)](https://pypi.python.org/pypi/clowder-repo) |

<details>

<summary>Why clowder</summary>

There are many ways to organize projects with git. Monorepos, submodules, subtrees, or [some](https://github.com/cristibalan/braid) [other](https://github.com/mixu/gr) [tool](https://github.com/ingydotnet/git-subrepo). `clowder` is one of the other tools. Its approach is heavily influeced by the [repo tool](https://gerrit.googlesource.com/git-repo) Google uses to manage the Android Open Source Project.

Projects information is stored in a `clowder.yml` file. If checked into its own repo, it can be shared among users. `clowder` essentially makes this file executable, allowing commands to be run across projects. `clowder` can update submodules, lfs files, and custom git config entries. Projects can track branches, or be tied to specific tags or commits. Upstreams can be configured along with their upstream source, wherever they may live. Snapshots of project states can be saved for later restoration. And more...

Daily development still takes place in individual repos, with normal `git` commands. But `clowder` is there if you need to synchronize or run commands on multiple repos.

</details>

<details>

<summary>Installation Instructions</summary>

Requirements:

* [git](https://git-scm.com)
* [Python 3](https://www.python.org/downloads/)

To install or upgrade `clowder` from PyPI:

```bash
sudo pip install clowder-repo --upgrade
```

To install the latest pre-release version:

```bash
sudo pip install clowder-repo --force-reinstall --pre
```

For command autocompletion, add to your shell profile:

```bash
command -v clowder >/dev/null 2>&1 && eval "$(register-python-argcomplete clowder)"
```

</details>

## clowder.yml

[Full clowder.yml syntax reference](docs/clowder-yml-syntax-reference.md)

An example `clowder.yml` for [some](https://github.com/llvm/llvm-project) [well](https://github.com/apple/swift)-[known](https://github.com/tensorflow/tensorflow) [projects](https://gerrit.googlesource.com/git-repo):

```yaml
name: cool-projects

clowder:
  - llvm/llvm-project
  - apple/swift
  - tensorflow/tensorflow
```

The name is simply a descriptive label. Projects are specified by the last components of the git clone url.

If the ommitted default settings are included:

```yaml
name: cool-projects

protocol: ssh

sources:
  github:
    url: github.com

defaults:
  remote: origin
  source: github
  branch: master

clowder:
  - name: llvm/llvm-project
    path: llvm-project
  - name: apple/swift
    path: swift
  - name: tensorflow/tensorflow
    path: tensorflow
```

The protocol specifies whether to use ssh or https for cloning repositories.

The sources section is where custom git hosting providers are specified. The following sources are built in:

| `source.name` |  `source.url`   |
| ------------- | --------------- |
| `github`      | `github.com`    |
| `gitlab`      | `gitlab.com`    |
| `bitbucket`   | `bitbucket.org` |

The defaults section contains the git branch, git remote name, and the source to clone from. If no branch is specified, the default remote branch is used. If no source is specified, the default is github.

A project requires at minimum a name (the last components of the git clone url). If the project path is not specified, the last component of the project name is used for the local directory.

Depending on the protocol, the project name is combined with the source url to form the full git clone url:

| protocol |                   git url                   |
| -------- | ------------------------------------------- |
| ssh      | `git@${source.url}:${project.name}.git`     |
| https    | `https://${source.url}/${project.name}.git` |

To add a project from a custom hosting location:

```yaml
name: cool-projects

sources:
  google:
    url: gerrit.googlesource.com
    protocol: https

clowder:
  - llvm/llvm-project
  - apple/swift
  - tensorflow/tensorflow
  - name: git-repo
    path: repo
    source: google
```

or equivalently:

```yaml
name: cool-projects

clowder:
  - llvm/llvm-project
  - apple/swift
  - tensorflow/tensorflow
  - name: git-repo
    path: repo
    source:
      url: gerrit.googlesource.com
      protocol: https
```

In order to be able to run commands for only certain sets of projects, there are groups:

```yaml
name: cool-projects

clowder:
  - name: llvm/llvm-project
    groups: [clattner]
  - name: apple/swift
    groups: [clattner]
  - name: tensorflow/tensorflow
    groups: [google, clattner]
  - name: git-repo
    path: repo
    groups: [google, notdefault]
    source:
      url: gerrit.googlesource.com
      protocol: https
```

or equivalently with sections:

```yaml
name: cool-projects

clowder:
  clattner:
    - llvm/llvm-project
    - apple/swift
    - name: tensorflow/tensorflow
      groups: [google]
  google:
    - name: git-repo
      path: repo
      groups: [notdefault]
      source:
        url: gerrit.googlesource.com
        protocol: https
```

or equivalently:

```yaml
name: cool-projects

sources:
  google:
    url: gerrit.googlesource.com
    protocol: https

clowder:
  clattner:
    - llvm/llvm-project
    - apple/swift
    - name: tensorflow/tensorflow
      groups: [google]
  google:
    defaults:
      source: google
    groups: [notdefault]
    projects:
      - name: git-repo
        path: repo
```

Projects are automatically added to the `all` group, and to groups of their name and path. If `notdefault` is specified, the project will not be included in commands unless it belongs to another supplied group argument.

To save projects to a subdirectory:

```yaml
name: cool-projects

clowder:
  clattner:
    path: clattner
    projects:
      - llvm/llvm-project
      - apple/swift
      - name: tensorflow/tensorflow
        groups: [google]
  google:
    path: google
    projects:
      - name: git-repo
        path: repo
        groups: [notdefault]
        source:
          url: gerrit.googlesource.com
          protocol: https
```

which is equivalent to:

```yaml
name: cool-projects

clowder:
  clattner:
    - name: llvm/llvm-project
      path: clattner/llvm-project
    - name: apple/swift
      path: clattner/swift
    - name: tensorflow/tensorflow
      path: clattner/tensorflow
      groups: [google]
  google:
    - name: git-repo
      path: google/repo
      groups: [notdefault]
      source:
        url: gerrit.googlesource.com
        protocol: https
```

For more examples, see the [clowder-examples repo](https://github.com/polka-dot-cat/clowder-examples/tree/master/versions)

## Command Usage

For the full command reference, see [the commands doc](docs/commands.md)

The following examples use an [existing repo](https://github.com/polka-dot-cat/clowder-examples) containing a [clowder.yml](https://github.com/polka-dot-cat/clowder-examples/blob/master/clowder.yml) file.

To follow along, create a directory where all the projects will be cloned:

```bash
mkdir cats
cd cats
```

<details>

<summary>clowder init</summary>

```bash
clowder init git@github.com:polka-dot-cat/clowder-examples.git
```

The `clowder init` command does the following:

* Clones the [examples clowder repo](https://github.com/polka-dot-cat/clowder-examples) in the `.clowder` directory
* Creates a symlink in the `cats` directory: `clowder.yml` -> `.clowder/clowder.yml`

![clowder init](docs/examples/clowder-init.gif)

</details>

<details>

<summary>clowder herd</summary>

The `clowder herd` command updates the state of the projects. When the command is run, the following happens:

* If any projects don't have a clean git status then `clowder` exits
* Projects are cloned if they don't currently exist
* Each project fetches the latest changes
* If the current git ref checked out doesn't match the `clowder.yml` configuration, the correct ref will be checked out
* The latest changes are pulled for branches. For commits and tags, the commits are checked out into a detached `HEAD` state

![clowder herd](docs/examples/clowder-herd.gif)

</details>

<details>

<summary>clowder status</summary>

The `clowder status` command prints the current state of all projects.

![clowder status](docs/examples/clowder-status.gif)

</details>

<details>

<summary>clowder forall</summary>

```bash
# Run command in all project directories
clowder forall -c 'git status'
```

</details>

<details>

<summary>clowder git commands</summary>

For more information, see [the commands doc](docs/commands.md#git-commands)

```bash
# Print all local branches
clowder branch

# Checkout 'my_branch' in projects if it exists
clowder checkout 'my_branch'

# Discard any changes in projects
clowder clean

# Print git diff for all projects
clowder diff

# Create new branch 'my_feature' for all projects
clowder start 'my_feature'

# Stash changes in all projects
clowder stash

# Prune branch 'stale_branch' for all projects
clowder prune 'stale_branch'
```

</details>

<details>

<summary>clowder repo commands</summary>

For more information, see [the commands doc](docs/commands.md#clowder-repo-commands)

```bash
# Set clowder.yml symlink to a previously saved version
clowder link 'v0.1'

# Run command in .clowder directory
clowder repo run 'git status'

# Save a version of clowder.yml with current commit sha's
clowder save 'v0.1'
```

</details>

<details>

<summary>clowder config commands</summary>

**_NOTE: EXPERIMENTAL_**

For more information, see [the commands doc](docs/commands.md#clowder-config)

```bash
# Get config values
clowder config get

# Set config values
clowder config set projects 'my_group'

# Clear config values
clowder config clear projects
```

</details>

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md)
