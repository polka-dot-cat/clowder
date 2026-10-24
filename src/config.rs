use crate::workspace::Workspace;
use anyhow::{Result, bail};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fs;

#[derive(Default)]
pub struct Config {
    sections: Map<String, Value>,
}

impl Config {
    pub fn load(workspace: &Workspace) -> Result<Self> {
        let path = workspace.root.join("clowder.config");
        Self::parse(&if path.is_file() {
            fs::read_to_string(path)?
        } else {
            String::new()
        })
    }

    fn parse(text: &str) -> Result<Self> {
        let mut result = Self::default();
        let mut section = String::new();
        let mut previous: Option<(String, usize)> = None;
        let mut seen_sections = HashSet::new();
        let mut seen_keys = HashSet::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(['#', ';']) {
                continue;
            }
            if trimmed.is_empty() {
                if let Some((key, _)) = &previous {
                    result.append(&section, key, "");
                }
                continue;
            }
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            if let Some((key, previous_indent)) = &previous
                && indent > *previous_indent
            {
                result.append(&section, key, trimmed);
                continue;
            }
            if let Some(header) = trimmed.strip_prefix('[')
                && let Some((name, _)) = header.split_once(']')
                && !name.is_empty()
            {
                section = name.to_owned();
                if !seen_sections.insert(section.clone()) && section != "DEFAULT" {
                    bail!("Duplicate clowder.config section: {section}");
                }
                result
                    .sections
                    .entry(section.clone())
                    .or_insert(Value::Object(Map::new()));
                previous = None;
            } else if let Some(index) = trimmed.find(['=', ':']) {
                if section.is_empty() {
                    bail!("Invalid clowder.config: setting outside a section");
                }
                let key = trimmed[..index].trim().to_lowercase();
                if key.is_empty() || !seen_keys.insert((section.clone(), key.clone())) {
                    bail!("Invalid or duplicate clowder.config key: {key}");
                }
                result.set(&section, &key, Some(trimmed[index + 1..].trim().into()));
                previous = Some((key, indent));
            } else {
                bail!("Invalid clowder.config line: {line}");
            }
        }
        for values in result.sections.values_mut() {
            for value in values.as_object_mut().unwrap().values_mut() {
                *value = Value::String(value.as_str().unwrap().trim_end().into());
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

    fn append(&mut self, section: &str, key: &str, line: &str) {
        if let Value::String(value) = &mut self.sections[section][key] {
            value.push('\n');
            value.push_str(line);
        }
    }

    fn raw(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .get(section)
            .and_then(|s| s.get(key))
            .or_else(|| self.sections.get("DEFAULT").and_then(|s| s.get(key)))?
            .as_str()
    }

    pub fn get(&self, section: &str, key: &str) -> Result<Option<String>> {
        self.raw(section, &key.to_lowercase())
            .map(|value| self.interpolate(section, value, 1))
            .transpose()
    }

    fn interpolate(&self, section: &str, mut text: &str, depth: usize) -> Result<String> {
        if depth > 10 {
            bail!("clowder.config interpolation is recursive or exceeds 10 levels");
        }
        let mut output = String::new();
        while let Some(index) = text.find('%') {
            output.push_str(&text[..index]);
            text = &text[index + 1..];
            if let Some(rest) = text.strip_prefix('%') {
                output.push('%');
                text = rest;
            } else if let Some(rest) = text.strip_prefix('(')
                && let Some((key, rest)) = rest.split_once(")s")
            {
                let value = self.raw(section, &key.to_lowercase()).ok_or_else(|| {
                    anyhow::anyhow!("Unknown clowder.config interpolation key: {key}")
                })?;
                output.push_str(&self.interpolate(section, value, depth + 1)?);
                text = rest;
            } else {
                bail!("Invalid clowder.config interpolation; use %% for a literal percent sign");
            }
        }
        output.push_str(text);
        Ok(output)
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

    pub fn projects(&self) -> Result<Vec<String>> {
        Ok(self
            .get("command", "projects")?
            .map(|s| s.trim().split(", ").map(str::to_owned).collect())
            .unwrap_or_default())
    }

    pub fn jobs(&self) -> Result<Option<i64>> {
        self.get("command", "jobs")?
            .map(|s| s.trim().replace('_', "").parse())
            .transpose()
            .map_err(Into::into)
    }

    pub fn rebase(&self) -> Result<Option<bool>> {
        match self
            .get("git", "rebase")?
            .map(|s| s.to_lowercase())
            .as_deref()
        {
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
        let defaults = self.sections.get_key_value("DEFAULT").into_iter();
        for (section, values) in
            defaults.chain(self.sections.iter().filter(|(name, _)| *name != "DEFAULT"))
        {
            if section == "DEFAULT" && values.as_object().unwrap().is_empty() {
                continue;
            }
            text += &format!("[{section}]\n");
            for (key, value) in values.as_object().unwrap() {
                text += &format!(
                    "{key} = {}\n",
                    value.as_str().unwrap().replace('\n', "\n\t")
                );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_python_config_parser_values_errors_and_serialization() {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/data/rust/config-compatibility.json"))
                .unwrap();
        let directory = tempfile::TempDir::new().unwrap();
        let workspace = Workspace {
            root: directory.path().into(),
            found: true,
        };
        for case in fixture["cases"].as_array().unwrap() {
            let text = case["ini"].as_str().unwrap();
            let result = Config::parse(text);
            if case["parse_error"] == true {
                assert!(result.is_err(), "Accepted invalid INI: {text}");
                continue;
            }
            let config = result.unwrap_or_else(|e| panic!("{text:?}: {e}"));
            for query in case["queries"].as_array().unwrap() {
                let actual = config.get(
                    query["section"].as_str().unwrap(),
                    query["key"].as_str().unwrap(),
                );
                if query["error"] == true {
                    assert!(actual.is_err(), "{text:?}, query: {query}");
                } else {
                    assert_eq!(
                        actual.unwrap().as_deref(),
                        query["value"].as_str(),
                        "{text:?}, query: {query}"
                    );
                }
            }
            config.save(&workspace).unwrap();
            assert_eq!(
                fs::read_to_string(workspace.root.join("clowder.config")).unwrap(),
                case["saved"].as_str().unwrap(),
                "{text:?}"
            );
        }
    }
}
