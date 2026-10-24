//! Completion reads the grammar and local workspace only. It never runs a
//! command handler or contacts a remote, including for incomplete arguments.
use super::{Action, Spec, option, specs};
use crate::{git, manifest::Manifest, workspace::Workspace};
use std::collections::BTreeSet;

fn action<'a>(spec: &'a Spec, flag: &str) -> Option<&'a Action> {
    if let Some(action) = spec
        .actions
        .iter()
        .find(|a| a.flags.iter().any(|f| f == flag))
    {
        return Some(action);
    }
    let mut matches = spec
        .actions
        .iter()
        .filter(|a| flag.starts_with("--") && a.flags.iter().any(|f| f.starts_with(flag)));
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn argument_values(command: &str, argument: &Action) -> Vec<String> {
    if let Some(choices) = &argument.choices {
        return choices.clone();
    }
    let Ok(workspace) = Workspace::discover() else {
        return Vec::new();
    };
    if argument.name == "version" && command == "link" {
        return std::fs::read_dir(workspace.repo().join("versions"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                name.strip_suffix(".clowder.yml")
                    .or_else(|| name.strip_suffix(".clowder.yaml"))
                    .map(str::to_owned)
            })
            .collect();
    }
    let manifest = Manifest::load(&workspace, None).ok();
    match argument.name.as_str() {
        "projects" => manifest
            .iter()
            .flat_map(|m| m.projects.iter())
            .flat_map(|p| p.groups.iter().cloned())
            .chain(std::iter::once("default".into()))
            .collect(),
        "exclude" => manifest
            .iter()
            .flat_map(|m| m.projects.iter().map(|p| p.name.clone()))
            .collect(),
        "branch" | "tag" | "ref" => {
            let paths = if command.starts_with("repo ") {
                vec![workspace.repo()]
            } else {
                manifest
                    .iter()
                    .flat_map(|m| m.projects.iter().map(|p| p.path.clone()))
                    .collect()
            };
            let refs: &[&str] = match argument.name.as_str() {
                "tag" => &["refs/tags"],
                "branch" => &["refs/heads"],
                _ => &["refs/heads", "refs/tags", "refs/remotes"],
            };
            paths
                .iter()
                .filter(|path| git::exists(path))
                .filter_map(|path| {
                    git::read(
                        path,
                        ["for-each-ref", "--format=%(refname:short)"]
                            .into_iter()
                            .chain(refs.iter().copied()),
                    )
                    .ok()
                })
                .flat_map(|text| text.lines().map(str::to_owned).collect::<Vec<_>>())
                .collect()
        }
        _ => Vec::new(),
    }
}

pub(super) fn candidates(tokens: &[String]) -> Vec<String> {
    let (current, previous) = tokens
        .split_last()
        .map_or(("", &[][..]), |(last, rest)| (last.as_str(), rest));
    let mut command = String::new();
    let mut pending: Option<&Action> = None;
    let mut positionals = 0;
    let mut options_ended = false;
    for token in previous {
        let spec = &specs()[&command];
        if let Some(argument) = pending {
            if !option(token) {
                if argument.nargs != "*" && argument.nargs != "+" {
                    pending = None;
                }
                continue;
            }
            pending = None;
        }
        if !options_ended && token == "--" {
            options_ended = true;
        } else if !options_ended && option(token) {
            if token.starts_with("--") {
                let (flag, attached) = token
                    .split_once('=')
                    .map_or((token.as_str(), false), |(flag, _)| (flag, true));
                pending = action(spec, flag).filter(|a| !a.boolean && !attached);
            } else {
                for (index, character) in token.char_indices().skip(1) {
                    let Some(argument) = action(spec, &format!("-{character}")) else {
                        break;
                    };
                    if !argument.boolean {
                        if index + character.len_utf8() == token.len() {
                            pending = Some(argument);
                        }
                        break;
                    }
                }
            }
        } else if !options_ended && spec.children.contains(token) && positionals == 0 {
            command = if command.is_empty() {
                token.clone()
            } else {
                format!("{command} {token}")
            };
        } else {
            positionals += 1;
        }
    }
    let spec = &specs()[&command];
    let values = if !options_ended && current.starts_with("--") && current.contains('=') {
        let (flag, prefix) = current.split_once('=').unwrap();
        action(spec, flag)
            .into_iter()
            .flat_map(|a| argument_values(&command, a))
            .filter(|value| value.starts_with(prefix))
            .map(|value| format!("{flag}={value}"))
            .collect()
    } else if !options_ended && current.starts_with('-') {
        spec.actions
            .iter()
            .flat_map(|a| a.flags.iter().cloned())
            .chain(std::iter::once("--help".into()))
            .chain(std::iter::once("-h".into()))
            .chain(command.is_empty().then_some("--version".into()))
            .collect()
    } else if let Some(argument) = pending {
        argument_values(&command, argument)
    } else if !spec.children.is_empty() {
        spec.children.clone()
    } else {
        let mut argument = None;
        for candidate in spec.actions.iter().filter(|a| a.flags.is_empty()) {
            if candidate.nargs == "*" || candidate.nargs == "+" || positionals == 0 {
                argument = Some(candidate);
                break;
            }
            positionals -= 1;
        }
        argument
            .map(|a| argument_values(&command, a))
            .unwrap_or_default()
    };
    values
        .into_iter()
        .filter(|value| value.starts_with(current))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
