//! Project operations own their output and subprocess environment so workers never
//! change the process working directory or mix their command output.
use crate::{
    cli::Args, commands::ExitCode, config::Config, git, manifest::Project, workspace::Workspace,
};
use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

#[derive(Default)]
struct Log(String);

impl Log {
    fn line(&mut self, message: impl AsRef<str>) {
        self.0.push_str(message.as_ref());
        self.0.push('\n');
    }

    fn git<I, S>(&mut self, path: &Path, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = git::output(path, args)?;
        self.0 += &String::from_utf8_lossy(&output.stdout);
        self.0 += &String::from_utf8_lossy(&output.stderr);
        if output.status.success() {
            Ok(())
        } else {
            Err(ExitCode(output.status.code().unwrap_or(1)).into())
        }
    }

    fn checkout(&mut self, project: &Project, reference: &str, check: bool) -> Result<()> {
        let reference = reference
            .strip_prefix("refs/heads/")
            .or_else(|| reference.strip_prefix("refs/tags/"))
            .unwrap_or(reference);
        if git::branch(&project.path).as_deref() == Some(reference) {
            self.line(format!(" - Branch {reference} already checked out"));
            return Ok(());
        }
        self.line(format!(" - Checkout branch {reference}"));
        let result = self.git(
            &project.path,
            ["-c", "advice.detachedHead=false", "checkout", reference],
        );
        if check {
            result
        } else {
            if result.is_err() {
                self.line(" - Failed to checkout");
            }
            Ok(())
        }
    }

    fn fetch(&mut self, project: &Project, remote: &str, check: bool) -> Result<()> {
        self.line(format!(" - Fetch {remote}"));
        let mut args = vec!["fetch".to_owned(), "--prune".into(), "--tags".into()];
        if let Some(depth) = project.git.depth {
            args.push(format!("--depth={depth}"));
        }
        args.push(remote.into());
        let result = self.git(&project.path, args);
        if check { result } else { Ok(()) }
    }

    fn lfs(&mut self, project: &Project) -> Result<()> {
        if project.git.lfs {
            self.line(" - Update git lfs hooks");
            // Install per-repository hooks without changing the user's global config.
            self.git(&project.path, ["lfs", "install", "--local"])?;
            self.line(" - Pull git lfs objects");
            self.git(&project.path, ["lfs", "pull"])?;
        }
        Ok(())
    }
}

pub fn run(
    args: &Args,
    workspace: &Workspace,
    config: &Config,
    projects: &[Project],
) -> Result<()> {
    let command = args.command.as_str();
    if ["herd", "reset", "prune", "start"].contains(&command) {
        for project in projects {
            git::validate(&project.path, true)?;
        }
    }
    if (command == "start" && args.flag("tracking"))
        || (command == "prune" && (args.flag("all") || args.flag("remote")))
    {
        // Check the actual remotes before creating or deleting local branches.
        // Local remotes remain usable without a public Internet connection.
        for project in projects.iter().filter(|p| git::exists(&p.path)) {
            git::read(&project.path, ["ls-remote", "--heads", &project.remote])?;
        }
    }
    let jobs = if ["herd", "reset", "forall"].contains(&command) {
        config.jobs()?.or(args.number("jobs")).unwrap_or(1)
    } else {
        1
    };
    let jobs = if jobs <= 0 { 4 } else { jobs as usize };
    let rebase = config.rebase()?.unwrap_or(args.flag("rebase"));
    let execute = |project: &Project| {
        let mut log = Log::default();
        let result = perform(args, workspace, project, rebase, &mut log)
            .with_context(|| format!("Project {}", project.name));
        (log.0, result)
    };
    if jobs == 1 {
        for project in projects {
            let (output, result) = execute(project);
            print!("{output}");
            result?;
        }
    } else {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs.min(projects.len().max(1)))
            .build()?;
        let results: Vec<_> = pool.install(|| projects.par_iter().map(execute).collect());
        let mut error = None;
        for (output, result) in results {
            print!("{output}");
            if error.is_none() {
                error = result.err();
            }
        }
        if let Some(error) = error {
            return Err(error);
        }
    }
    Ok(())
}

fn perform(
    args: &Args,
    workspace: &Workspace,
    project: &Project,
    rebase: bool,
    log: &mut Log,
) -> Result<()> {
    log.line(project.status(None)?);
    if args.command != "herd" && !git::exists(&project.path) {
        log.line(" - Project missing\n");
        return Ok(());
    }
    let path = &project.path;
    match args.command.as_str() {
        "herd" => herd(args, project, rebase, log),
        "checkout" => {
            log.checkout(project, args.one("branch").unwrap(), false)?;
            log.lfs(project)
        }
        "forall" => {
            let command = args.one("command").unwrap();
            log.line(command);
            let mut process = Command::new("sh");
            process
                .args(["-c", command])
                .current_dir(path)
                .env("CLOWDER_PATH", &workspace.root)
                .env("PROJECT_PATH", path)
                .env("PROJECT_NAME", &project.name)
                .env("PROJECT_REMOTE", &project.remote)
                .env("PROJECT_REF", project.default_ref());
            if let Some(upstream) = &project.upstream {
                process
                    .env("UPSTREAM_NAME", &upstream.name)
                    .env("UPSTREAM_REMOTE", &upstream.remote);
            } else {
                process
                    .env_remove("UPSTREAM_NAME")
                    .env_remove("UPSTREAM_REMOTE");
            }
            let result = process.output()?;
            log.0 += &String::from_utf8_lossy(&result.stdout);
            log.0 += &String::from_utf8_lossy(&result.stderr);
            if !result.status.success() && !args.flag("ignore_errors") {
                return Err(ExitCode(result.status.code().unwrap_or(1)).into());
            }
            Ok(())
        }
        "branch" => {
            if args.flag("remote") || args.flag("all") {
                log.fetch(project, &project.remote, false)?;
                if let Some(upstream) = &project.upstream {
                    log.fetch(project, &upstream.remote, false)?;
                }
            }
            if !args.flag("remote") {
                let branches = git::read(
                    path,
                    ["for-each-ref", "--format=%(refname:short)", "refs/heads"],
                )?;
                let current = git::branch(path);
                for branch in branches.lines() {
                    log.line(if current.as_deref() == Some(branch) {
                        format!("* {branch}")
                    } else {
                        branch.to_owned()
                    });
                }
            }
            // The Python CLI does not currently print remote branch details.
            Ok(())
        }
        "diff" => log.git(path, ["status", "-vv"]),
        "stash" => {
            if git::dirty(path) {
                log.line(" - Stash current changes");
                log.git(path, ["stash"])
            } else {
                log.line(" - No changes to stash");
                Ok(())
            }
        }
        "clean" => {
            let all = args.flag("all");
            let mut flags = "-f".to_owned();
            if all || args.flag("untracked_directories") {
                flags.push('d');
            }
            if all || args.flag("force") {
                flags.push('f');
            }
            if !all && args.flag("ignored") {
                flags.push('X');
            }
            if all || args.flag("untracked_files") {
                flags.push('x');
            }
            log.line(" - Clean repo");
            log.git(path, ["clean", &flags])?;
            log.line(" - Reset repo to HEAD");
            log.git(path, ["reset", "--hard", "HEAD"])?;
            let git_dir = path.join(git::read(path, ["rev-parse", "--git-dir"])?);
            if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
                log.git(path, ["rebase", "--abort"])?;
            }
            if args.flag("submodules") && !all {
                bail!("Cleaning submodules is not implemented");
            }
            Ok(())
        }
        "reset" => {
            log.fetch(project, &project.remote, false)?;
            if project.tag.is_none() && project.commit.is_none() {
                let branch = project.default_branch();
                if !git::ref_exists(path, &format!("refs/heads/{branch}")) {
                    return log.git(path, ["branch", "--track", &branch, "HEAD"]);
                }
                log.checkout(project, &branch, true)?;
                let upstream = format!("{}/{branch}", project.remote);
                if !git::ref_exists(path, &format!("refs/remotes/{upstream}")) {
                    bail!("No existing upstream branch {upstream}");
                }
                log.line(format!(
                    " - Reset branch {branch} to {} {branch}",
                    project.remote
                ));
                log.line(format!(" - Reset repo to {upstream}"));
                log.git(path, ["reset", "--hard", &upstream])?;
            }
            log.lfs(project)
        }
        "start" => {
            let branch = args.one("branch").unwrap();
            if !git::ref_exists(path, &format!("refs/heads/{branch}")) {
                log.line(format!(" - Create local branch {branch}"));
                log.git(path, ["branch", "--no-track", branch, "HEAD"])?;
            }
            log.checkout(project, branch, true)?;
            if args.flag("tracking") {
                log.fetch(project, &project.remote, true)?;
                let remote = format!("{}/{branch}", project.remote);
                if !git::ref_exists(path, &format!("refs/remotes/{remote}")) {
                    log.line(format!(" - Create remote branch {branch}"));
                    log.git(
                        path,
                        [
                            "push",
                            &project.remote,
                            &format!("refs/heads/{branch}:refs/heads/{branch}"),
                        ],
                    )?;
                    log.fetch(project, &project.remote, true)?;
                }
                log.git(path, ["branch", "--set-upstream-to", &remote, branch])?;
            }
            Ok(())
        }
        "prune" => {
            let branch = args.one("branch").unwrap();
            let local = !args.flag("remote");
            let remote = args.flag("remote") || args.flag("all");
            if local && git::branch(path).as_deref() == Some(branch) {
                log.checkout(project, &project.default_ref(), false)?;
            }
            if local && git::ref_exists(path, &format!("refs/heads/{branch}")) {
                log.line(format!(" - Delete local branch {branch}"));
                log.git(
                    path,
                    [
                        "branch",
                        if args.flag("force") { "-D" } else { "-d" },
                        branch,
                    ],
                )?;
            }
            if remote && git::ref_exists(path, &format!("refs/remotes/{}/{branch}", project.remote))
            {
                log.line(format!(" - Delete remote branch {branch}"));
                log.git(path, ["push", &project.remote, "--delete", branch])?;
            }
            Ok(())
        }
        other => bail!("Unknown command: {other}"),
    }
}

fn herd(args: &Args, project: &Project, rebase: bool, log: &mut Log) -> Result<()> {
    let path = &project.path;
    let initial = !git::exists(path);
    if initial {
        if path.exists() {
            if path.read_dir()?.next().is_some() {
                bail!("Non-empty directory already exists: {}", path.display());
            }
            std::fs::remove_dir(path)?;
        }
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut clone_branch = project.branch.clone();
        if let Some(branch) = args.one("branch") {
            let refs = git::read(
                path.parent().unwrap(),
                [
                    "ls-remote",
                    "--heads",
                    &project.url,
                    &format!("refs/heads/{branch}"),
                ],
            )?;
            if !refs.is_empty() {
                clone_branch = Some(branch.into());
            }
        }
        let mut flags = vec![
            "clone".to_owned(),
            "--origin".into(),
            project.remote.clone(),
        ];
        if let Some(branch) = clone_branch {
            flags.extend(["--branch".into(), branch]);
        }
        if let Some(depth) = args
            .number("depth")
            .map(|n| n.to_string())
            .or_else(|| project.git.depth.map(|n| n.to_string()))
        {
            flags.extend(["--depth".into(), depth]);
        }
        flags.extend([project.url.clone(), path.to_string_lossy().into_owned()]);
        log.line(" - Clone repo");
        log.git(path.parent().unwrap(), flags)?;
        if let Some(tag) = args.one("tag") {
            if git::ref_exists(path, &format!("refs/tags/{tag}")) {
                log.checkout(project, tag, true)?;
            }
        } else if let Some(reference) = project.tag.as_ref().or(project.commit.as_ref()) {
            log.checkout(project, reference, true)?;
        }
    }
    log.line(" - Update git herd alias");
    // Git runs aliases beginning with '!' through a shell. Keep the manifest path
    // as one literal argument, and put it after '--' so leading '-' is harmless.
    // Forward user flags before that separator (for example, git herd --rebase).
    let selector = project.relative.to_string_lossy().replace('\'', "'\\''");
    let alias = format!("!f() {{ clowder herd \"$@\" -- '{selector}'; }}; f");
    log.git(path, ["config", "--local", "alias.herd", &alias])?;
    for (key, value) in &project.git.config {
        let value = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        log.git(path, ["config", "--local", key, &value])?;
    }
    if !initial {
        if !git::succeeds(path, ["remote", "get-url", &project.remote]) {
            log.git(path, ["remote", "add", &project.remote, &project.url])?;
        }
        log.fetch(project, &project.remote, true)?;
        if let Some(reference) = project.tag.as_ref().or(project.commit.as_ref()) {
            log.checkout(project, reference, true)?;
        } else {
            herd_branch(project, &project.default_branch(), true, rebase, log)?;
        }
        if let Some(branch) = args.one("branch") {
            herd_branch(project, branch, false, rebase, log)?;
        } else if let Some(tag) = args.one("tag")
            && git::ref_exists(path, &format!("refs/tags/{tag}"))
        {
            log.checkout(project, tag, true)?;
        }
    }
    log.lfs(project)?;
    if let Some(upstream) = &project.upstream {
        log.line(&upstream.name);
        if !git::succeeds(path, ["remote", "get-url", &upstream.remote]) {
            log.git(path, ["remote", "add", &upstream.remote, &upstream.url])?;
        }
        log.fetch(project, &upstream.remote, true)?;
    }
    if project.git.recursive {
        log.line(" - Update submodules");
        let mut flags = vec![
            "submodule".to_owned(),
            "update".into(),
            "--init".into(),
            "--recursive".into(),
            "--checkout".into(),
        ];
        if let Some(depth) = project.git.depth {
            flags.push(format!("--depth={depth}"));
        }
        log.git(path, flags)?;
    }
    Ok(())
}

fn herd_branch(
    project: &Project,
    branch: &str,
    create: bool,
    rebase: bool,
    log: &mut Log,
) -> Result<()> {
    let path = &project.path;
    let upstream = format!("{}/{branch}", project.remote);
    let has_upstream = git::ref_exists(path, &format!("refs/remotes/{upstream}"));
    if !git::ref_exists(path, &format!("refs/heads/{branch}")) {
        if has_upstream {
            return log.git(path, ["checkout", "--track", &upstream]);
        }
        if create {
            return log.git(path, ["branch", "--track", branch, "HEAD"]);
        }
        return Ok(());
    }
    log.checkout(project, branch, create)?;
    if !git::ref_exists(path, &format!("{branch}@{{upstream}}")) {
        log.git(path, ["branch", "--set-upstream-to", &upstream, branch])?;
    }
    log.line(format!(
        " - Pull{} from {} {branch}",
        if rebase { " with rebase" } else { "" },
        project.remote
    ));
    let mut flags = vec!["pull", "--no-edit"];
    if rebase {
        flags.push("--rebase");
    }
    flags.extend([&project.remote, branch]);
    log.git(path, flags)
}
