//! Git subprocesses keep credentials, config, and transport behavior in Git itself.
use anyhow::{Context, Result, bail};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

pub fn output<I, S>(path: &Path, args: I) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .with_context(|| format!("Failed to run Git in {}", path.display()))
}

pub fn read<I, S>(path: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let result = output(path, args)?;
    if !result.status.success() {
        bail!("{}", String::from_utf8_lossy(&result.stderr).trim_end());
    }
    Ok(String::from_utf8_lossy(&result.stdout)
        .trim_end()
        .to_string())
}

pub fn run<I, S>(path: &Path, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let result = output(path, args)?;
    print!("{}", String::from_utf8_lossy(&result.stdout));
    eprint!("{}", String::from_utf8_lossy(&result.stderr));
    if !result.status.success() {
        bail!(
            "Git exited with status {}",
            result.status.code().unwrap_or(1)
        );
    }
    Ok(())
}

pub fn succeeds<I, S>(path: &Path, args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    output(path, args).is_ok_and(|output| output.status.success())
}

pub fn exists(path: &Path) -> bool {
    path.join(".git").exists() && succeeds(path, ["rev-parse", "--verify", "HEAD"])
}

pub fn dirty(path: &Path) -> bool {
    read(path, ["status", "--porcelain", "--untracked-files=no"]).is_ok_and(|s| !s.is_empty())
}

pub fn validate(path: &Path, allow_missing: bool) -> Result<()> {
    if !exists(path) {
        if allow_missing {
            return Ok(());
        }
        bail!(
            "Project missing: {}. First run clowder herd",
            path.display()
        );
    }
    let changes = read(path, ["status", "--porcelain"])?;
    let git_dir = read(path, ["rev-parse", "--git-dir"])?;
    let git_dir = path.join(git_dir);
    if !changes.is_empty()
        || git_dir.join("rebase-merge").exists()
        || git_dir.join("rebase-apply").exists()
    {
        bail!(
            "{}\n{changes}\nDirty repo. Please stash, commit, or discard your changes",
            path.display()
        );
    }
    // Validate initialized submodules even when status ignores their changes.
    if path.join(".gitmodules").is_file()
        && let Ok(entries) = read(
            path,
            [
                "config",
                "--file",
                ".gitmodules",
                "--get-regexp",
                r"^submodule\..*\.path$",
            ],
        )
    {
        for entry in entries.lines() {
            if let Some((_, relative)) = entry.split_once(' ') {
                validate(&path.join(relative), allow_missing)?;
            }
        }
    }
    Ok(())
}

pub fn branch(path: &Path) -> Option<String> {
    read(path, ["symbolic-ref", "--quiet", "--short", "HEAD"]).ok()
}

pub fn formatted_ref(path: &Path) -> Result<String> {
    let Some(branch) = branch(path) else {
        return Ok(format!("[HEAD @ {}]", read(path, ["rev-parse", "HEAD"])?));
    };
    let mut result = format!("[{branch}]");
    if let Ok(counts) = read(
        path,
        ["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
    ) {
        let numbers: Vec<_> = counts.split_whitespace().collect();
        if numbers.len() == 2 && numbers != ["0", "0"] {
            result += &format!("(+{}/-{})", numbers[0], numbers[1]);
        }
    }
    Ok(result)
}

pub fn ref_exists(path: &Path, reference: &str) -> bool {
    succeeds(path, ["rev-parse", "--verify", "--quiet", reference])
}

pub fn default_branch(path: &Path, remote: &str, url: &str) -> String {
    if exists(path)
        && let Ok(reference) = read(
            path,
            [
                "symbolic-ref",
                "--short",
                &format!("refs/remotes/{remote}/HEAD"),
            ],
        )
    {
        return reference
            .strip_prefix(&format!("{remote}/"))
            .unwrap_or(&reference)
            .to_owned();
    }
    let cwd = if path.is_dir() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default()
    };
    if let Ok(refs) = read(&cwd, ["ls-remote", "--symref", url, "HEAD"]) {
        for line in refs.lines() {
            if let Some(reference) = line.strip_prefix("ref: refs/heads/") {
                return reference
                    .split_whitespace()
                    .next()
                    .unwrap_or("master")
                    .to_owned();
            }
        }
    }
    "master".into()
}
