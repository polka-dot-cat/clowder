//! Benchmark-only module injected into a disposable source copy by the driver.
//! This is not a production cache implementation or part of the shipped CLI.
use crate::{config::Config, manifest::Manifest, workspace::Workspace};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, hint::black_box, io::Write, path::Path, time::Instant};

#[derive(Serialize, Deserialize, PartialEq)]
struct Stamp {
    size: u64,
    modified_ns: u128,
}

impl Stamp {
    fn read(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?;
        Ok(Self {
            size: metadata.len(),
            modified_ns: metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos(),
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Entry {
    format: u32,
    hash: String,
    stamp: Stamp,
    raw: Value,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn load(workspace: &Workspace, protocol: Option<&str>) -> Result<Manifest> {
    let mode = std::env::var("CLOWDER_BENCH_MODE").unwrap_or_else(|_| "baseline".into());
    let path = workspace.manifest()?;
    if mode == "baseline" {
        let text =
            fs::read_to_string(&path).with_context(|| format!("Cannot read {}", path.display()))?;
        let raw = crate::yaml::parse(&text).context("Invalid YAML")?;
        return Manifest::from_value(workspace, raw, protocol);
    }
    if !["json", "validated", "stat"].contains(&mode.as_str()) {
        bail!("Unknown benchmark mode: {mode}");
    }
    let directory = std::env::var_os("CLOWDER_BENCH_CACHE").context("Benchmark cache directory")?;
    let directory = Path::new(&directory);
    let stamp = Stamp::read(&path)?;
    let bytes = if mode == "stat" {
        None
    } else {
        Some(fs::read(&path)?)
    };
    let hash = bytes.as_deref().map(digest);
    // Each run has a separate cache directory; the driver records its source
    // fingerprint. The metadata-only prototype demonstrates mtime/size limits.
    let key = if mode == "stat" {
        digest(path.as_os_str().as_encoded_bytes())
    } else {
        hash.clone().unwrap()
    };
    let cache = directory.join(format!("{key}.json"));
    if let Ok(encoded) = fs::read(&cache)
        && let Ok(entry) = serde_json::from_slice::<Entry>(&encoded)
        && entry.format == 1
        && if mode == "stat" {
            entry.stamp == stamp
        } else {
            Some(&entry.hash) == hash.as_ref()
        }
    {
        return if mode == "json" {
            Manifest::from_value(workspace, entry.raw, protocol)
        } else {
            Manifest::from_validated_value(workspace, entry.raw, protocol)
        };
    }
    let bytes = match bytes {
        Some(bytes) => bytes,
        None => fs::read(&path)?,
    };
    let hash = hash.unwrap_or_else(|| digest(&bytes));
    let text = std::str::from_utf8(&bytes)?;
    let raw = crate::yaml::parse(text).context("Invalid YAML")?;
    if let Err(error) = crate::manifest::schema().validate(&raw) {
        bail!("Invalid clowder YAML: {error}");
    }
    let entry = Entry {
        format: 1,
        hash,
        stamp,
        raw,
    };
    let encoded = serde_json::to_vec(&entry)?;
    // Atomic replacement, with no fsync: cache persistence is not guaranteed.
    // Fail on write errors here so the benchmark cannot silently measure a miss.
    fs::create_dir_all(directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(&encoded)?;
    temporary.persist(cache)?;
    Manifest::from_validated_value(workspace, entry.raw, protocol)
}

fn workspace() -> Result<(Workspace, Option<String>)> {
    let workspace = Workspace::discover()?;
    let config = Config::load(&workspace)?;
    let protocol = config.get("git", "protocol")?;
    Ok((workspace, protocol))
}

/// Fresh-process stage measurements; nothing here changes the actual CLI.
pub fn action(action: &str) -> Result<()> {
    if action == "load" || action == "inspect" {
        let (workspace, protocol) = workspace()?;
        let manifest = Manifest::load(&workspace, protocol.as_deref())?;
        if action == "load" {
            println!("{}", black_box(manifest.projects.len()));
        } else {
            let projects: Vec<_> = manifest
                .projects
                .iter()
                .map(|p| json!({"name":p.name,"path":p.path,"url":p.url}))
                .collect();
            println!("{}", json!({"name":manifest.name,"projects":projects}));
        }
        return Ok(());
    }
    if action != "profile" {
        bail!("Unknown benchmark action: {action}");
    }
    let start = Instant::now();
    let (workspace, protocol) = workspace()?;
    let discovery_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    let text = fs::read_to_string(workspace.manifest()?)?;
    let read_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    let raw = crate::yaml::parse(&text)?;
    let yaml_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    let validator = crate::manifest::schema();
    let schema_init_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    validator
        .validate(&raw)
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let validation_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    let manifest = Manifest::from_validated_value(&workspace, raw, protocol.as_deref())?;
    let resolve_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    let encoded = serde_json::to_vec(&manifest.raw)?;
    let json_encode_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    black_box(serde_json::from_slice::<Value>(&encoded)?);
    let json_decode_us = start.elapsed().as_secs_f64() * 1e6;
    let start = Instant::now();
    black_box(digest(text.as_bytes()));
    let hash_us = start.elapsed().as_secs_f64() * 1e6;
    println!(
        "{}",
        json!({
            "discovery_config_us":discovery_us,"read_us":read_us,"yaml_us":yaml_us,
            "schema_init_us":schema_init_us,"validation_us":validation_us,
            "resolve_us":resolve_us,"json_encode_us":json_encode_us,
            "json_decode_us":json_decode_us,"hash_us":hash_us,
            "projects":manifest.projects.len(),"yaml_bytes":text.len(),"json_bytes":encoded.len()
        })
    );
    Ok(())
}
