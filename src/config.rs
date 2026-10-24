use crate::workspace::Workspace;
use anyhow::{Result, bail};
use serde_json::{Map, Value};
use std::fs;

#[derive(Default)]
pub struct Config {
    sections: Map<String, Value>,
}

impl Config {
    pub fn load(workspace: &Workspace) -> Result<Self> {
        let mut result = Self::default();
        let path = workspace.root.join("clowder.config");
        if path.is_file() {
            let mut section = String::new();
            for line in fs::read_to_string(path)?.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with(['#', ';']) {
                    continue;
                }
                if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                    section = name.to_owned();
                    result
                        .sections
                        .entry(section.clone())
                        .or_insert(Value::Object(Map::new()));
                } else if let Some((key, value)) =
                    line.split_once('=').or_else(|| line.split_once(':'))
                {
                    if section.is_empty() {
                        bail!("Invalid clowder.config: setting outside a section");
                    }
                    result.set(&section, key.trim(), Some(value.trim().to_owned()));
                } else {
                    bail!("Invalid clowder.config line: {line}");
                }
            }
        }
        for section in ["git", "command"] {
            result
                .sections
                .entry(section.to_owned())
                .or_insert(Value::Object(Map::new()));
        }
        Ok(result)
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections.get(section)?.get(key)?.as_str()
    }

    pub fn set(&mut self, section: &str, key: &str, value: Option<String>) {
        let section = self
            .sections
            .entry(section.to_owned())
            .or_insert(Value::Object(Map::new()))
            .as_object_mut()
            .unwrap();
        if let Some(value) = value {
            section.insert(key.to_lowercase(), Value::String(value));
        } else {
            section.shift_remove(key);
        }
    }

    pub fn projects(&self) -> Vec<String> {
        self.get("command", "projects")
            .map(|s| s.split(", ").map(str::to_owned).collect())
            .unwrap_or_default()
    }

    pub fn jobs(&self) -> Result<Option<i64>> {
        self.get("command", "jobs")
            .map(str::parse)
            .transpose()
            .map_err(Into::into)
    }

    pub fn rebase(&self) -> Result<Option<bool>> {
        match self.get("git", "rebase").map(str::to_lowercase).as_deref() {
            None => Ok(None),
            Some("true" | "yes" | "on" | "1") => Ok(Some(true)),
            Some("false" | "no" | "off" | "0") => Ok(Some(false)),
            Some(value) => bail!("Invalid rebase setting: {value}"),
        }
    }

    pub fn save(&self, workspace: &Workspace) -> Result<()> {
        if !workspace.found {
            bail!("No clowder config file path found");
        }
        let mut text = String::new();
        for (section, values) in &self.sections {
            text += &format!("[{section}]\n");
            for (key, value) in values.as_object().unwrap() {
                text += &format!("{key} = {}\n", value.as_str().unwrap());
            }
            text.push('\n');
        }
        fs::write(workspace.root.join("clowder.config"), text)?;
        Ok(())
    }

    pub fn print(workspace: &Workspace) -> Result<()> {
        let path = workspace.root.join("clowder.config");
        if path.is_file() {
            println!("Current config\n\n{}\n", fs::read_to_string(path)?.trim());
        } else {
            println!(" - No config file found");
        }
        Ok(())
    }
}
