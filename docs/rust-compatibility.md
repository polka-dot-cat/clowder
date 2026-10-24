# Rust compatibility notes

The native executable implements all 17 top-level commands and their existing
subcommands. It reads the same manifests, resolves source/default/section/project
settings, uses the existing schema, and operates through the installed Git CLI.
The Python implementation remains in the checkout for comparison and for utilities
used by the unchanged BDD harness. Native commands do not invoke it.

## Preserved behavior

- Project names, paths, sections, groups, exclusions, and `notdefault` selection.
- Manifest aliases and merge keys, YAML 1.1 scalar types, both YAML extensions,
  snapshots, and relative workspace links.
- INI configuration precedence, defaults, interpolation, multiline values, and
  case-insensitive option names.
- Branch, tag, commit, upstream, shallow-clone, recursive-submodule, and LFS flows
  covered by the existing scenarios.
- Dirty-tree checks before branch mutations, including initialized submodules.
- Per-project shell variables and command exit codes. Each worker uses its own
  current directory and environment.

The [validation record](rust-migration-validation.md) gives case counts and the
limits of each environment. The existing 1,111 Gherkin cases are unchanged.

## Differences to review

The native CLI uses plain text and concise error chains. It does not reproduce
Rich's ANSI styling, progress display, Python tracebacks, or source-file links.
Representative plain help, version, status, YAML, and configuration output is
checked against captured Python output.

Sequential `forall` commands inherit stdin, stdout, and stderr so shell commands
can read input and show progress. Parallel commands receive closed stdin and
spool combined stdout/stderr to private temporary files. Results are printed in
project order after the workers finish, and the files are removed. This keeps
output from different projects together and avoids retaining large shell logs
in memory. If several commands fail, the first failed project in that order
determines the exit code; `--ignore-errors` still succeeds.

Remote preflight checks contact the configured Git remotes instead of probing an
unrelated public connectivity endpoint. Local remotes remain usable offline.
The native `git herd` alias quotes paths and forwards options safely. Duplicate
paths are checked after resolving symlinks to prevent concurrent operations on
the same repository through different aliases.

The manifest representation uses string mapping keys and finite JSON-compatible
numbers. Integers must fit signed or unsigned 64-bit values. Quote dates when
using them as strings. Recursive YAML aliases are rejected; nesting and alias
expansion have limits documented in the migration guide.

Python `argcomplete` registration is replaced by native Bash and Zsh completion
scripts; see the installation guide. Completion reads the local workspace and
Git refs without running a command handler or contacting remotes. Windows is
not a validated target; the current migration targets macOS and Linux.

## Existing limitations retained

This port keeps several behaviors of the reference CLI that need separate fixes:

- `config clear` recreates the current settings after removing the file. Clearing
  an individual setting works. Fixing the all-settings command is in `TODO.md`.
- Remote-only `branch` fetches refs without printing remote branch details.
- Nonrecursive submodule update/cleanup behavior remains incomplete.
- `reset` does not yet restore a tag/commit-pinned project that moved elsewhere.
- `herd` creates missing remotes but does not reconcile changed URLs of existing
  remotes.

Those changes need their own regression cases and release notes. The migration
does not claim to fix the full backlog by replacing the implementation language.
