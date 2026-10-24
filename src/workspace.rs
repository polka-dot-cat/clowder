use crate::git;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub found: bool,
}

pub fn present(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

pub fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if result.file_name().is_some_and(|name| name != "..") {
                    result.pop();
                } else if !result.has_root() {
                    result.push("..");
                }
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

impl Workspace {
    pub fn discover() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        for path in cwd.ancestors() {
            if [".clowder", "clowder.yml", "clowder.yaml"]
                .iter()
                .any(|name| present(&path.join(name)))
            {
                return Ok(Self {
                    root: path.to_path_buf(),
                    found: true,
                });
            }
        }
        Ok(Self {
            root: cwd,
            found: false,
        })
    }

    pub fn repo(&self) -> PathBuf {
        self.root.join(".clowder")
    }

    pub fn require_repo(&self, is_git: bool) -> Result<PathBuf> {
        let path = self.repo();
        if !path.is_dir() {
            bail!("No .clowder directory found");
        }
        if is_git && !git::exists(&path) {
            bail!("No .clowder git repository found");
        }
        Ok(path)
    }

    pub fn manifest(&self) -> Result<PathBuf> {
        let yml = self.root.join("clowder.yml");
        let yaml = self.root.join("clowder.yaml");
        if present(&yml) && present(&yaml) {
            bail!("Found clowder.yml and clowder.yaml files in same directory");
        }
        let path = if present(&yaml) { yaml } else { yml };
        if !path.is_file() {
            bail!("{} appears to be missing", path.display());
        }
        Ok(path)
    }

    pub fn status(&self) -> Result<String> {
        let repo = self.repo();
        if !repo.is_dir() {
            return Ok(String::new());
        }
        let mut result = ".clowder".to_owned();
        if git::exists(&repo) {
            if git::dirty(&repo) {
                result.push('*');
            }
            result.push(' ');
            result += &git::formatted_ref(&repo)?;
        }
        if let Ok(path) = self.manifest()
            && path.is_symlink()
        {
            let source = fs::canonicalize(&path)?;
            result += &format!(
                "\n{} -> {}",
                path.file_name().unwrap().to_string_lossy(),
                source.strip_prefix(&self.root).unwrap_or(&source).display()
            );
        }
        result.push('\n');
        Ok(result)
    }

    pub fn link(&self, version: Option<&str>) -> Result<()> {
        let repo = self.require_repo(false)?;
        let (directory, stem) = match version {
            Some(version) => (repo.join("versions"), format!("{version}.clowder")),
            None => (repo, "clowder".into()),
        };
        if let Ok(entries) = fs::read_dir(self.repo().join("versions")) {
            let mut names = std::collections::HashSet::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some(name) = name
                    .strip_suffix(".clowder.yml")
                    .or_else(|| name.strip_suffix(".clowder.yaml"))
                    && !names.insert(name.to_owned())
                {
                    bail!("Duplicate version found: {name}");
                }
            }
        }
        let yml = directory.join(format!("{stem}.yml"));
        let yaml = directory.join(format!("{stem}.yaml"));
        let source = if yml.is_file() {
            yml
        } else if yaml.is_file() {
            yaml
        } else {
            bail!("Symlink source {} appears to be missing", yml.display());
        };
        let extension = source.extension().unwrap().to_str().unwrap();
        let target = self.root.join(format!("clowder.{extension}"));
        if present(&target) && !target.is_symlink() {
            bail!("Found non-symlink file {} at target path", target.display());
        }
        for name in ["clowder.yml", "clowder.yaml"] {
            let path = self.root.join(name);
            if path.is_symlink() {
                fs::remove_file(path)?;
            }
        }
        let relative = source.strip_prefix(&self.root).unwrap_or(&source);
        println!(
            " - Symlink {} -> {}",
            target.file_name().unwrap().to_string_lossy(),
            relative.display()
        );
        symlink_file(relative, &target)
            .with_context(|| format!("Failed to link {}", target.display()))
    }
}

#[cfg(unix)]
fn symlink_file(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn symlink_file(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(source, target)
}
