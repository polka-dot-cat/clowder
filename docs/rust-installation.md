# Native Clowder installation

The Rust migration produces one `clowder` executable for macOS or Linux.
It needs Git on `PATH`, a POSIX shell for `forall` and `repo run`, and Git LFS
only when a manifest enables LFS. Python is needed only for the retained test
harness and the old reference implementation.

## Build from source

Install the Rust toolchain selected by `rust-toolchain.toml`, then run:

```sh
cargo build --release --locked
./target/release/clowder --version
```

To install into Cargo's binary directory:

```sh
cargo install --path . --locked
```

Keep the Cargo binary directory on `PATH`. If a Python release is also installed,
use `command -v clowder` to check which executable your shell selects. During
evaluation, use an absolute path to the native binary to avoid that ambiguity.
The Python `register-python-argcomplete` setup does not apply to this executable.

## Shell completion

Native completion supports commands, flags, choices, workspace projects/groups,
saved versions, and local Git refs. It does not contact remote repositories.
Source the matching file from this checkout or an extracted archive in your
shell profile, using its actual path:

```sh
# Bash (.bashrc or .bash_profile)
source /path/to/clowder/completions/clowder.bash

# Zsh (.zshrc), after your existing compinit setup
autoload -Uz compinit
compinit
source /path/to/clowder/completions/clowder.zsh
```

Use the native `clowder` executable on `PATH`. These scripts replace the old
Python argcomplete registration; they do not need Python. Shells retain normal
file completion when the native helper has no candidates.

## Install a native archive

Native archives are built separately for each operating system and architecture.
No native release has been published yet. Choose an archive matching your
machine, check its SHA-256 file, and extract it:

```sh
# Substitute the actual archive name in both commands.
shasum -a 256 -c clowder-VERSION-TARGET.tar.gz.sha256
tar -xzf clowder-VERSION-TARGET.tar.gz
```

Linux also supports `sha256sum -c` for the checksum. Copy the extracted `clowder`
to a directory on your `PATH`, or run it by its absolute path. A user-owned
directory such as `~/.local/bin` avoids a system installation:

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 clowder-VERSION-TARGET/clowder "$HOME/.local/bin/clowder"
"$HOME/.local/bin/clowder" --version
```

The binary uses the same manifests, `clowder.config`, Git credentials, and
repositories as the Python release. No workspace conversion is required.

## Build an archive for review or release

From a clean checkout, run the local checks and package the host binary:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
script/package-rust
```

`script/package-rust` writes a `.tar.gz` and SHA-256 file under `dist/rust/`.
The archive contains the binary, shell completion files, this guide, the project
license, and dependency notices. It does not upload files, create a tag, or publish
a release. An optional target triple selects a
different installed Rust target; its linker and system libraries must also be
available. Build on each target platform unless a cross toolchain is configured.

After changing Rust dependencies, regenerate `THIRD-PARTY-NOTICES.md` before
packaging. The archive helper checks that its recorded lockfile digest matches.
Notice generation uses the pinned [cargo-about](https://embarkstudios.github.io/cargo-about/)
tool; normal builds and archive packaging do not require it:

```sh
cargo install cargo-about --version 0.9.2 --features cli --locked
script/update-rust-licenses
```

Review the generated notices and any new license types with dependency updates.

GNU Linux binaries depend on the build system's glibc baseline. A macOS binary
depends on its build deployment target. Record those minimum operating system
versions with any published assets and test the extracted archive on each
claimed platform. Use a musl build when a static Linux executable is required.

Before publishing, finish the compatibility and CI checks recorded in
`docs/rust-migration-validation.md`, choose the native release version, and
update Cargo metadata and CLI version together. Keep the native release separate
from the legacy `script/deploy` workflow, which still builds and uploads Python
packages. Publishing, package-manager updates, and signing remain release tasks.
