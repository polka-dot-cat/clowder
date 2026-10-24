use crate::{cli::Args, config::Config, git, manifest::Manifest, workspace::Workspace};
use anyhow::{Result, bail};
use std::fs;

#[derive(Debug)]
pub struct ExitCode(pub i32);
impl std::fmt::Display for ExitCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command exited with status {}", self.0)
    }
}
impl std::error::Error for ExitCode {}

pub fn run(args: &Args) -> Result<()> {
    let workspace = Workspace::discover()?;
    if args.command == "init" {
        let workspace = Workspace {
            root: std::env::current_dir()?,
            found: true,
        };
        let repo = workspace.repo();
        if repo.is_dir() {
            fs::remove_dir(&repo)?;
        }
        let url = args.one("url").unwrap();
        println!("Create clowder repo from {url}\n");
        let mut options = vec!["clone", "--origin", "origin"];
        if let Some(branch) = args.one("branch") {
            options.extend(["--branch", branch]);
        }
        options.extend([url, ".clowder"]);
        git::run(&workspace.root, options)?;
        workspace.link(None)?;
        workspace.manifest()?;
        return Ok(());
    }
    if args.command == "link" {
        if let Ok(manifest) = Manifest::load(&workspace, None) {
            println!("{}\n", manifest.name);
        }
        print!("{}", workspace.status()?);
        return workspace.link(args.one("version"));
    }
    if args.command == "repo" || args.command == "config set" {
        crate::cli::help(&args.command);
        return Ok(());
    }
    let mut config = Config::load(&workspace)?;
    if args.command.starts_with("repo ") {
        return repo(args, &workspace);
    }
    let protocol = config.get("git", "protocol").or(args.one("protocol"));
    let manifest = Manifest::load(&workspace, protocol)?;
    if args.command == "yaml" {
        print!("{}", manifest.yaml(args.flag("resolved"))?);
        return Ok(());
    }
    println!("{}\n", manifest.name);
    if args.command.starts_with("config") {
        let parts: Vec<_> = args.command.split(' ').collect();
        if parts.len() >= 2 {
            let clear = parts[1] == "clear";
            if parts.len() == 2 {
                println!(" - Clear all config values");
                // Retain the established clear command's serialization behavior.
                config.save(&workspace)?;
            } else {
                let key = parts[2];
                if key == "projects" && !clear {
                    manifest.selected(&args.many("projects"), &[], &[])?;
                }
                println!(
                    " - {} {key} config value",
                    if clear { "Clear" } else { "Set" }
                );
                let section = if ["jobs", "projects"].contains(&key) {
                    "command"
                } else {
                    "git"
                };
                let value = if clear {
                    None
                } else if key == "rebase" {
                    Some("True".to_owned())
                } else if key == "projects" {
                    Some(args.many(key).join(", "))
                } else {
                    args.one(key).map(str::to_owned)
                };
                config.set(section, key, value);
                config.save(&workspace)?;
            }
        }
        return Config::print(&workspace);
    }
    if args.command == "save" {
        workspace.require_repo(false)?;
        let version = args
            .one("version")
            .unwrap()
            .to_lowercase()
            .replace('/', "-");
        if version == "default" {
            bail!("Version name 'default' is not allowed");
        }
        print!("{}", workspace.status()?);
        let yaml = manifest.yaml(true)?;
        let versions = workspace.repo().join("versions");
        let path = versions.join(format!("{version}.clowder.yml"));
        if path.exists() || versions.join(format!("{version}.clowder.yaml")).exists() {
            bail!("Version '{version}' already exists");
        }
        fs::create_dir_all(versions)?;
        println!(" - Save version '{version}'\n{}", path.display());
        fs::write(path, yaml)?;
        return Ok(());
    }
    let projects = manifest.selected(
        &args.many("projects"),
        &config.projects(),
        &args.many("exclude"),
    )?;
    if args.command == "status" {
        if args.flag("fetch") {
            if git::exists(&workspace.repo()) {
                git::run(&workspace.repo(), ["fetch", "--prune", "--tags", "origin"])?;
            }
            for project in &projects {
                if git::exists(&project.path) {
                    git::run(
                        &project.path,
                        ["fetch", "--prune", "--tags", &project.remote],
                    )?;
                    if let Some(upstream) = &project.upstream {
                        git::run(
                            &project.path,
                            ["fetch", "--prune", "--tags", &upstream.remote],
                        )?;
                    }
                }
            }
        }
        print!("{}", workspace.status()?);
        let width = projects
            .iter()
            .map(|p| p.display_name().chars().count())
            .max()
            .unwrap_or(0);
        for project in &projects {
            println!("{}", project.status(Some(width))?);
        }
        return Ok(());
    }
    print!("{}", workspace.status()?);
    crate::operations::run(args, &workspace, &config, &projects)
}

fn repo(args: &Args, workspace: &Workspace) -> Result<()> {
    let path = workspace.require_repo(true)?;
    if let Ok(manifest) = Manifest::load(workspace, None) {
        println!("{}\n", manifest.name);
    }
    print!("{}", workspace.status()?);
    match args.command.as_str() {
        "repo add" => {
            let mut flags = vec!["add".to_owned()];
            flags.extend(args.many("files"));
            git::run(&path, flags)
        }
        "repo status" => git::run(&path, ["status", "-vv"]),
        "repo checkout" => {
            if git::dirty(&path) {
                println!(" - Dirty repo. Please stash, commit, or discard your changes");
                git::run(&path, ["status", "-vv"])
            } else {
                git::run(
                    &path,
                    [
                        "-c",
                        "advice.detachedHead=false",
                        "checkout",
                        args.one("ref").unwrap(),
                    ],
                )
            }
        }
        "repo commit" => git::run(&path, ["commit", "-m", args.one("message").unwrap()]),
        "repo pull" => git::run(&path, ["pull", "--no-edit"]),
        "repo push" => git::run(&path, ["push"]),
        "repo clean" => git::run(&path, ["clean", "-f"]),
        "repo run" => {
            let command = args.one("command").unwrap();
            println!("{command}");
            let status = std::process::Command::new("sh")
                .args(["-c", command])
                .current_dir(path)
                .status()?;
            if status.success() {
                Ok(())
            } else {
                Err(ExitCode(status.code().unwrap_or(1)).into())
            }
        }
        _ => unreachable!(),
    }
}
