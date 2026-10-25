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
