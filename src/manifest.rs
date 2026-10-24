//! Manifest validation and inheritance are pure; loading a manifest never fetches Git refs.
use crate::git;
use crate::workspace::{Workspace, normalize};
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Map, Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Upstream {
    pub name: String,
    pub remote: String,
    pub url: String,
}

#[derive(Debug, Default, Clone)]
pub struct GitSettings {
    pub submodules: bool,
    pub recursive: bool,
    pub lfs: bool,
    pub depth: Option<u64>,
    pub config: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub relative: PathBuf,
    pub path: PathBuf,
    pub remote: String,
    pub url: String,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub commit: Option<String>,
    pub upstream: Option<Upstream>,
    pub groups: BTreeSet<String>,
    pub git: GitSettings,
}

impl Project {
    pub fn default_branch(&self) -> String {
        self.branch
            .clone()
            .unwrap_or_else(|| git::default_branch(&self.path, &self.remote, &self.url))
    }

    pub fn default_ref(&self) -> String {
        if let Some(branch) = &self.branch {
            format!("refs/heads/{branch}")
        } else if let Some(tag) = &self.tag {
            format!("refs/tags/{tag}")
        } else if let Some(commit) = &self.commit {
            commit.clone()
        } else {
            format!("refs/heads/{}", self.default_branch())
        }
    }

    pub fn display_name(&self) -> String {
        if !git::exists(&self.path) {
            self.relative.to_string_lossy().into_owned()
        } else {
            format!(
                "{}{}",
                self.name,
                if git::dirty(&self.path) { "*" } else { "" }
            )
        }
    }

    pub fn status(&self, width: Option<usize>) -> Result<String> {
        let name = self.display_name();
        let name = match width {
            Some(width) => format!("{name:width$}"),
            None => name,
        };
        if !git::exists(&self.path) {
            return Ok(if width.is_some() {
                format!("{name} -")
            } else {
                name
            });
        }
        Ok(format!("{name} {}", git::formatted_ref(&self.path)?))
    }
}

pub struct Manifest {
    pub name: String,
    pub raw: Value,
    pub projects: Vec<Project>,
}

fn schema() -> &'static jsonschema::Validator {
    static SCHEMA: OnceLock<jsonschema::Validator> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let value: Value = serde_json::from_str(include_str!("../clowder/clowder.schema.json"))
            .expect("valid embedded schema");
        jsonschema::validator_for(&value).expect("valid Clowder schema")
    })
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_owned)
}

fn inherited<'a>(
    key: &str,
    project: &'a Value,
    section: &'a Value,
    defaults: &'a Value,
) -> Option<&'a Value> {
    [project, section, defaults]
        .iter()
        .find_map(|value| value.get(key).filter(|v| !v.is_null()))
}

fn source_url(
    source: Option<&Value>,
    sources: &HashMap<String, Value>,
    root_protocol: Option<&str>,
    override_protocol: Option<&str>,
    name: &str,
) -> Result<String> {
    let github = Value::String("github".into());
    let source = source.unwrap_or(&github);
    let definition = if let Some(alias) = source.as_str() {
        sources
            .get(alias)
            .ok_or_else(|| anyhow!("Unknown source: {alias}"))?
    } else {
        source
    };
    let host = definition["url"]
        .as_str()
        .ok_or_else(|| anyhow!("Source is missing its URL"))?;
    let protocol = definition["protocol"]
        .as_str()
        .or(override_protocol)
        .or(root_protocol)
        .unwrap_or("ssh");
    match protocol {
        "ssh" => Ok(format!("git@{host}:{name}.git")),
        "https" => Ok(format!("https://{host}/{name}.git")),
        _ => bail!("Unknown protocol: {protocol}"),
    }
}

impl Manifest {
    pub fn load(workspace: &Workspace, protocol: Option<&str>) -> Result<Self> {
        let path = workspace.manifest()?;
        let contents =
            fs::read_to_string(&path).with_context(|| format!("Cannot read {}", path.display()))?;
        let raw = crate::yaml::parse(&contents).context("Invalid YAML")?;
        Self::from_value(workspace, raw, protocol)
    }

    pub fn from_value(workspace: &Workspace, raw: Value, protocol: Option<&str>) -> Result<Self> {
        if let Err(error) = schema().validate(&raw) {
            bail!("Invalid clowder YAML: {error}");
        }
        let mut sources = HashMap::from([
            ("github".into(), json!({"url":"github.com"})),
            ("gitlab".into(), json!({"url":"gitlab.com"})),
            ("bitbucket".into(), json!({"url":"bitbucket.org"})),
        ]);
        if let Some(custom) = raw["sources"].as_object() {
            sources.extend(
                custom
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone())),
            );
        }
        let defaults = &raw["defaults"];
        let root_protocol = raw["protocol"].as_str();
        let mut entries = Vec::new();
        if let Some(projects) = raw["clowder"].as_array() {
            for project in projects {
                entries.push((project, None, &Value::Null));
            }
        } else if let Some(sections) = raw["clowder"].as_object() {
            for (section_name, section) in sections {
                let projects = section
                    .as_array()
                    .or_else(|| section["projects"].as_array())
                    .unwrap();
                for project in projects {
                    entries.push((project, Some(section_name), section));
                }
            }
        }
        let mut projects = Vec::new();
        let mut paths = HashSet::new();
        for (project, section_name, section) in entries {
            let name = project
                .as_str()
                .or_else(|| project["name"].as_str())
                .unwrap()
                .to_owned();
            let section_defaults = &section["defaults"];
            let property = |key| inherited(key, project, section_defaults, defaults);
            let path_name = project["path"]
                .as_str()
                .unwrap_or_else(|| name.rsplit('/').next().unwrap());
            let relative =
                normalize(&PathBuf::from(section["path"].as_str().unwrap_or("")).join(path_name));
            let path = normalize(&workspace.root.join(&relative));
            if !paths.insert(path.clone()) {
                bail!("Multiple projects with path '{}'", path.display());
            }
            let remote = property("remote")
                .and_then(Value::as_str)
                .unwrap_or("origin")
                .to_owned();
            let url = source_url(property("source"), &sources, root_protocol, protocol, &name)?;
            let mut groups = BTreeSet::from([
                "all".to_owned(),
                name.clone(),
                relative.to_string_lossy().into_owned(),
            ]);
            if let Some(section_name) = section_name {
                groups.insert(section_name.clone());
            }
            for value in [project, section] {
                if let Some(names) = value["groups"].as_array() {
                    groups.extend(names.iter().filter_map(Value::as_str).map(str::to_owned));
                }
            }
            if groups.contains("notdefault") {
                groups.remove("all");
            }
            let mut settings = GitSettings::default();
            for git in [&defaults["git"], &section_defaults["git"], &project["git"]] {
                if let Some(submodules) = git.get("submodules") {
                    settings.submodules = submodules == true || submodules == "recursive";
                    settings.recursive = submodules == "recursive";
                }
                if let Some(lfs) = git["lfs"].as_bool() {
                    settings.lfs = lfs;
                }
                if let Some(depth) = git["depth"].as_u64() {
                    settings.depth = Some(depth);
                }
                if let Some(config) = git["config"].as_object() {
                    for (key, value) in config {
                        if value.is_null() {
                            settings.config.shift_remove(key);
                        } else {
                            settings.config.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
            let branch = property("branch")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let tag = if branch.is_none() {
                property("tag").and_then(Value::as_str).map(str::to_owned)
            } else {
                None
            };
            let commit = if branch.is_none() && tag.is_none() {
                property("commit")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            } else {
                None
            };
            let upstream = if let Some(upstream) = project.get("upstream") {
                let name = upstream
                    .as_str()
                    .or_else(|| upstream["name"].as_str())
                    .unwrap()
                    .to_owned();
                let section_upstream = &section_defaults["upstream"];
                let root_upstream = &defaults["upstream"];
                let property = |key| inherited(key, upstream, section_upstream, root_upstream);
                let upstream_remote = property("remote")
                    .and_then(Value::as_str)
                    .unwrap_or("upstream")
                    .to_owned();
                if upstream_remote == remote {
                    bail!("Upstream '{name}' and project have same remote name '{remote}'");
                }
                Some(Upstream {
                    url: source_url(property("source"), &sources, root_protocol, protocol, &name)?,
                    name,
                    remote: upstream_remote,
                })
            } else {
                None
            };
            projects.push(Project {
                name,
                relative,
                path,
                remote,
                url,
                branch,
                tag,
                commit,
                upstream,
                groups,
                git: settings,
            });
        }
        projects.sort_by_key(|p| p.name.to_lowercase());
        Ok(Self {
            name: raw["name"].as_str().unwrap().to_owned(),
            raw,
            projects,
        })
    }

    pub fn selected(
        &self,
        names: &[String],
        defaults: &[String],
        exclude: &[String],
    ) -> Result<Vec<Project>> {
        let names = if names.is_empty() || names == ["default"] {
            defaults
        } else {
            names
        };
        let all = vec!["all".to_owned()];
        let names = if names.is_empty() { &all } else { names };
        for name in names {
            if !self
                .projects
                .iter()
                .any(|project| project.groups.contains(name))
            {
                bail!("Unknown project or group: {name}");
            }
        }
        Ok(self
            .projects
            .iter()
            .filter(|project| {
                !exclude.contains(&project.name)
                    && names.iter().any(|name| project.groups.contains(name))
            })
            .cloned()
            .collect())
    }

    pub fn yaml(&self, resolved: bool) -> Result<String> {
        let mut raw = self.raw.clone();
        if resolved {
            for project in &self.projects {
                git::validate(&project.path, false)?;
            }
        }
        fn project_yaml(
            value: &Value,
            manifest: &Manifest,
            resolved: bool,
            section_path: &str,
        ) -> Result<Value> {
            if value.is_string() && !resolved {
                return Ok(value.clone());
            }
            let name = value.as_str().or_else(|| value["name"].as_str()).unwrap();
            let mut result = Map::new();
            result.insert("name".into(), Value::String(name.into()));
            if let Some(path) = value.get("path") {
                result.insert("path".into(), path.clone());
            }
            if resolved {
                let relative = normalize(
                    &PathBuf::from(section_path).join(
                        value["path"]
                            .as_str()
                            .unwrap_or_else(|| name.rsplit('/').next().unwrap()),
                    ),
                );
                let project = manifest
                    .projects
                    .iter()
                    .find(|p| p.name == name && p.relative == relative)
                    .unwrap();
                result.insert(
                    "commit".into(),
                    Value::String(git::read(&project.path, ["rev-parse", "HEAD"])?),
                );
            } else {
                for key in ["branch", "tag", "commit"] {
                    if let Some(value) = value.get(key) {
                        result.insert(key.into(), value.clone());
                    }
                }
            }
            for key in ["groups", "remote", "source", "git", "upstream"] {
                if let Some(value) = value.get(key) {
                    result.insert(key.into(), canonical(key, value));
                }
            }
            Ok(Value::Object(result))
        }
        if let Some(projects) = raw["clowder"].as_array_mut() {
            for project in projects {
                *project = project_yaml(project, self, resolved, "")?;
            }
        } else if let Some(sections) = raw["clowder"].as_object_mut() {
            for section in sections.values_mut() {
                let path = text(section, "path").unwrap_or_default();
                let projects = if section.is_array() {
                    section.as_array_mut().unwrap()
                } else {
                    section["projects"].as_array_mut().unwrap()
                };
                for project in projects {
                    *project = project_yaml(project, self, resolved, &path)?;
                }
                if section.is_object() {
                    *section = canonical("section", section);
                }
            }
        }
        crate::yaml::stringify(&canonical("root", &raw))
    }
}

fn canonical(kind: &str, value: &Value) -> Value {
    if !value.is_object() {
        return value.clone();
    }
    let keys: &[&str] = match kind {
        "root" => &["name", "protocol", "sources", "defaults", "clowder"],
        "defaults" => &[
            "branch", "tag", "commit", "remote", "source", "git", "upstream",
        ],
        "git" => &["submodules", "lfs", "depth", "config"],
        "source" => &["url", "protocol"],
        "section" => &["path", "groups", "defaults", "projects"],
        "upstream" => &["name", "remote", "source"],
        _ => return value.clone(),
    };
    let mut result = Map::new();
    for key in keys {
        if let Some(value) = value.get(key) {
            let value = if *key == "sources" {
                Value::Object(
                    value
                        .as_object()
                        .unwrap()
                        .iter()
                        .map(|(k, v)| (k.clone(), canonical("source", v)))
                        .collect(),
                )
            } else {
                canonical(key, value)
            };
            result.insert((*key).to_owned(), value);
        }
    }
    Value::Object(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_section_overrides_and_keeps_upstream_defaults_independent() {
        let workspace = Workspace {
            root: PathBuf::from("/tmp/workspace"),
            found: true,
        };
        let raw = serde_yaml_ng::from_str(
            r#"
name: sample
protocol: https
defaults:
  remote: origin
  source: gitlab
  git: {submodules: recursive, config: {sample.old: yes, sample.keep: 4}}
  upstream: {remote: upstream, source: github}
clowder:
  tools:
    path: tools
    groups: [utilities]
    defaults:
      remote: fork
      git: {config: {sample.old: null}}
    projects:
    - name: owner/tool
      source: {url: example.org, protocol: ssh}
      upstream: other/tool
"#,
        )
        .unwrap();
        let manifest = Manifest::from_value(&workspace, raw, Some("https")).unwrap();
        let project = &manifest.projects[0];
        assert_eq!(project.url, "git@example.org:owner/tool.git");
        assert_eq!(project.relative, PathBuf::from("tools/tool"));
        assert_eq!(project.remote, "fork");
        assert!(project.git.recursive);
        assert!(!project.git.config.contains_key("sample.old"));
        assert_eq!(project.git.config["sample.keep"], 4);
        assert_eq!(
            project.upstream.as_ref().unwrap().url,
            "https://github.com/other/tool.git"
        );
        assert!(project.groups.contains("utilities"));
    }

    #[test]
    fn rejects_duplicate_paths_and_schema_violations() {
        let workspace = Workspace {
            root: PathBuf::from("/tmp/workspace"),
            found: true,
        };
        assert!(
            Manifest::from_value(
                &workspace,
                json!({"name":"x","clowder":["one/repo","two/repo"]}),
                None
            )
            .is_err()
        );
        assert!(
            Manifest::from_value(
                &workspace,
                json!({"name":"x","clowder":[{"name":"one/repo","branch":"main","tag":"v1"}]}),
                None
            )
            .is_err()
        );
    }
}
