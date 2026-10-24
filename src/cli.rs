//! The legacy command grammar, independent of manifest loading and Git state.
use anyhow::{Result, anyhow, bail};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::OnceLock;

mod completion;

pub fn complete(tokens: &[String]) -> Vec<String> {
    completion::candidates(tokens)
}

#[derive(Debug, Deserialize)]
struct Action {
    name: String,
    flags: Vec<String>,
    nargs: Value,
    boolean: bool,
    integer: bool,
    default: Value,
    choices: Option<Vec<String>>,
    metavar: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Spec {
    help: String,
    usage: String,
    actions: Vec<Action>,
    children: Vec<String>,
    exclusive: Vec<Vec<String>>,
}

fn specs() -> &'static BTreeMap<String, Spec> {
    static SPECS: OnceLock<BTreeMap<String, Spec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        serde_json::from_str(include_str!("cli_spec.json")).expect("valid CLI grammar")
    })
}

pub fn help(command: &str) {
    let mut help = specs()[command].help.clone();
    if let Ok(workspace) = crate::workspace::Workspace::discover()
        && let Ok(manifest) = crate::manifest::Manifest::load(&workspace, None)
        && let Some(line) = help
            .lines()
            .find(|line| line.starts_with("  <project|group>"))
            .map(str::to_owned)
    {
        let table = project_table(&manifest);
        if !table.is_empty() {
            let mut replacement = format!("{line}:\n                        \n");
            for row in table {
                replacement += &format!("                        {row}\n");
            }
            help = help.replacen(&format!("{line}\n"), &replacement, 1);
        }
    }
    print!("{help}");
}

fn project_table(manifest: &crate::manifest::Manifest) -> Vec<String> {
    fn sorted(values: impl Iterator<Item = String>) -> Vec<String> {
        let mut values: Vec<_> = values
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        values.sort_by_key(|value| value.to_lowercase());
        values
    }
    let names = sorted(manifest.projects.iter().map(|p| p.name.clone()));
    let paths = sorted(
        manifest
            .projects
            .iter()
            .map(|p| p.relative.to_string_lossy().into_owned()),
    );
    let upstreams = sorted(
        manifest
            .projects
            .iter()
            .filter_map(|p| p.upstream.as_ref().map(|u| u.name.clone())),
    );
    let groups = sorted(
        manifest
            .projects
            .iter()
            .flat_map(|p| p.groups.iter().cloned())
            .filter(|name| {
                !names.contains(name) && !paths.contains(name) && !upstreams.contains(name)
            }),
    );
    if names.is_empty() || paths.is_empty() || groups.is_empty() {
        return Vec::new();
    }
    let width = |values: &[String]| values.iter().map(|s| s.chars().count()).max().unwrap_or(1);
    let rows = |columns: &[(&[String], usize)], titles: &[&str]| {
        let mut rows = vec![
            columns
                .iter()
                .zip(titles)
                .map(|((_, width), title)| format!("{title:width$}"))
                .collect::<String>(),
            columns
                .iter()
                .zip(titles)
                .map(|((_, width), title)| format!("{:width$}", "-".repeat(title.len())))
                .collect(),
        ];
        for index in 0..columns
            .iter()
            .map(|(values, _)| values.len())
            .max()
            .unwrap_or(0)
        {
            rows.push(
                columns
                    .iter()
                    .map(|(values, width)| {
                        format!(
                            "{:width$}",
                            values.get(index).map(String::as_str).unwrap_or("")
                        )
                    })
                    .collect(),
            );
        }
        rows
    };
    if upstreams.is_empty() {
        rows(
            &[
                (&names, width(&names) + 2),
                (&paths, width(&paths) + 2),
                (&groups, width(&groups)),
            ],
            &["Project Names", "Project Paths", "Project Groups"],
        )
    } else {
        let name_width = width(&names).max(width(&upstreams)) + 2;
        let other_width = width(&paths).max(width(&groups));
        let mut result = rows(
            &[(&names, name_width), (&paths, other_width)],
            &["Project Names", "Project Paths"],
        );
        result.push(String::new());
        result.extend(rows(
            &[(&upstreams, name_width), (&groups, other_width)],
            &["Upstream Names", "Project Groups"],
        ));
        result
    }
}

#[derive(Debug, Default)]
pub struct Args {
    pub command: String,
    pub values: HashMap<String, Vec<String>>,
    pub debug: bool,
}

impl Args {
    pub fn flag(&self, key: &str) -> bool {
        self.one(key) == Some("true")
    }

    pub fn one(&self, key: &str) -> Option<&str> {
        self.values.get(key)?.first().map(String::as_str)
    }

    pub fn many(&self, key: &str) -> Vec<String> {
        self.values.get(key).cloned().unwrap_or_default()
    }

    pub fn number(&self, key: &str) -> Option<i64> {
        self.one(key)?.parse().ok()
    }
}

fn option(token: &str) -> bool {
    token.starts_with('-') && token != "-" && token.parse::<i64>().is_err()
}

fn error(spec: &Spec, command: &str, message: impl std::fmt::Display) -> anyhow::Error {
    let program = if command.is_empty() {
        "clowder".into()
    } else {
        format!("clowder {command}")
    };
    anyhow!("{}\n{program}: error: {message}", spec.usage.trim_end())
}

fn validate(action: &Action, value: &str) -> Result<()> {
    if action.integer && value.parse::<i64>().is_err() {
        bail!("argument {}: invalid int value: '{value}'", action.name);
    }
    if let Some(choices) = &action.choices
        && !choices.iter().any(|choice| choice == value)
    {
        bail!(
            "argument {}: invalid choice: '{value}' (choose from {})",
            action.name,
            choices
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

pub fn parse(tokens: Vec<String>) -> Result<Option<Args>> {
    let mut pending: VecDeque<String> = tokens.into();
    let mut args = Args::default();
    let mut positional = Vec::new();
    let mut options_ended = false;
    let mut seen = Vec::new();
    while let Some(mut token) = pending.pop_front() {
        let spec = &specs()[&args.command];
        if !options_ended {
            if token.starts_with("--") && token != "--" {
                let (flag, suffix) = token
                    .split_once('=')
                    .map_or((token.as_str(), None), |(flag, value)| (flag, Some(value)));
                let mut flags: Vec<_> = spec
                    .actions
                    .iter()
                    .flat_map(|a| a.flags.iter().map(String::as_str))
                    .collect();
                flags.push("--help");
                if args.command.is_empty() {
                    flags.push("--version");
                }
                if !flags.contains(&flag) {
                    let matches: Vec<_> = flags
                        .into_iter()
                        .filter(|candidate| candidate.starts_with(flag))
                        .collect();
                    if matches.len() > 1 {
                        return Err(error(
                            spec,
                            &args.command,
                            format!(
                                "ambiguous option: {flag} could match {}",
                                matches.join(", ")
                            ),
                        ));
                    }
                    if let Some(resolved) = matches.first() {
                        token = suffix.map_or_else(
                            || (*resolved).to_owned(),
                            |value| format!("{resolved}={value}"),
                        );
                    }
                }
            }
            if token == "--" {
                options_ended = true;
                continue;
            }
            if token == "-h" || token == "--help" {
                help(&args.command);
                return Ok(None);
            }
            if args.command.is_empty() && (token == "-v" || token == "--version") {
                println!("clowder version 4.0b7");
                return Ok(None);
            }
            if !spec.children.is_empty() && !option(&token) && positional.is_empty() {
                if !spec.children.contains(&token) {
                    return Err(error(
                        spec,
                        &args.command,
                        format!("invalid command '{token}'"),
                    ));
                }
                args.command = if args.command.is_empty() {
                    token
                } else {
                    format!("{} {token}", args.command)
                };
                seen.clear();
                continue;
            }
            if option(&token) {
                let mut attached = None;
                if token.starts_with("--") {
                    if let Some((flag, value)) = token.split_once('=') {
                        attached = Some(value.to_owned());
                        token = flag.to_owned();
                    }
                } else if token.len() > 2 && token.is_ascii() {
                    let flag = token[..2].to_owned();
                    if let Some(action) = spec.actions.iter().find(|a| a.flags.contains(&flag)) {
                        let rest = token[2..].to_owned();
                        if action.boolean {
                            pending.push_front(format!("-{rest}"));
                        } else {
                            attached = Some(rest);
                        }
                        token = flag;
                    }
                }
                let matches: Vec<_> = spec
                    .actions
                    .iter()
                    .filter(|a| a.flags.contains(&token))
                    .collect();
                if matches.len() != 1 {
                    return Err(error(
                        spec,
                        &args.command,
                        format!("unrecognized arguments: {token}"),
                    ));
                }
                let action = matches[0];
                seen.push(action.name.clone());
                if action.boolean {
                    if attached.is_some() {
                        return Err(error(
                            spec,
                            &args.command,
                            format!("argument {token}: ignored explicit argument"),
                        ));
                    }
                    args.values.insert(action.name.clone(), vec!["true".into()]);
                    if action.name == "debug" {
                        args.debug = true;
                    }
                } else {
                    let mut values = Vec::new();
                    if let Some(value) = attached {
                        values.push(value);
                    }
                    let multiple = action.nargs == "+" || action.nargs == "*";
                    while let Some(value) = pending.front() {
                        if option(value) || (!multiple && !values.is_empty()) {
                            break;
                        }
                        values.push(pending.pop_front().unwrap());
                    }
                    if values.is_empty() && action.nargs != "*" {
                        return Err(error(
                            spec,
                            &args.command,
                            format!("argument {token}: expected one argument"),
                        ));
                    }
                    for value in &values {
                        validate(action, value).map_err(|e| error(spec, &args.command, e))?;
                    }
                    args.values.insert(action.name.clone(), values);
                }
                continue;
            }
        }
        positional.push(token);
    }
    let spec = &specs()[&args.command];
    if args.command.is_empty() {
        eprint!("{}", spec.help);
        return Ok(None);
    }
    for group in &spec.exclusive {
        let used: Vec<_> = group.iter().filter(|name| seen.contains(name)).collect();
        if used.len() > 1 {
            return Err(error(
                spec,
                &args.command,
                format!(
                    "argument {}: not allowed with argument {}",
                    used[1], used[0]
                ),
            ));
        }
    }
    let mut remaining = positional.into_iter();
    for action in &spec.actions {
        if !action.flags.is_empty() {
            continue;
        }
        let multiple = action.nargs == "+" || action.nargs == "*";
        let optional = action.nargs == "?" || action.nargs == "*";
        let values: Vec<_> = if multiple {
            remaining.by_ref().collect()
        } else {
            remaining.next().into_iter().collect()
        };
        if values.is_empty() {
            if !optional {
                return Err(error(
                    spec,
                    &args.command,
                    format!(
                        "the following arguments are required: {}",
                        action.metavar.as_ref().unwrap_or(&action.name)
                    ),
                ));
            }
            if let Some(default) = action.default.as_str() {
                args.values
                    .insert(action.name.clone(), vec![default.to_owned()]);
            }
        } else {
            for value in &values {
                validate(action, value).map_err(|e| error(spec, &args.command, e))?;
            }
            args.values.insert(action.name.clone(), values);
        }
    }
    let excess: Vec<_> = remaining.collect();
    if !excess.is_empty() {
        return Err(error(
            spec,
            &args.command,
            format!("unrecognized arguments: {}", excess.join(" ")),
        ));
    }
    Ok(Some(args))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tokens: &[&str]) -> Result<Args> {
        parse(tokens.iter().map(|s| s.to_string()).collect()).map(Option::unwrap)
    }

    #[test]
    fn preserves_quoted_shell_commands_and_combined_flags() {
        let result = args(&["forall", "-ij4", "printf '%s' \"$PROJECT_NAME\"", "cats"]).unwrap();
        assert!(result.flag("ignore_errors"));
        assert_eq!(result.number("jobs"), Some(4));
        assert_eq!(result.one("command"), Some("printf '%s' \"$PROJECT_NAME\""));
        assert_eq!(result.many("projects"), vec!["cats"]);
    }

    #[test]
    fn validates_values_conflicts_and_missing_arguments() {
        assert!(args(&["herd", "-b", "main", "-t", "v1"]).is_err());
        assert!(args(&["herd", "--jobs", "many"]).is_err());
        assert!(args(&["config", "set", "protocol", "ftp"]).is_err());
        assert!(args(&["checkout"]).is_err());
        assert_eq!(
            args(&["config", "set", "jobs", "-1"])
                .unwrap()
                .number("jobs"),
            Some(-1)
        );
    }
}
