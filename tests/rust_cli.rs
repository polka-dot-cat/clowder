//! Black-box tests use only the native executable and temporary Git repositories.
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::TempDir;

fn clowder(path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clowder"))
        .args(args)
        .current_dir(path)
        .output()
        .unwrap()
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn git(path: &Path, args: &[&str]) -> String {
    success(
        Command::new("git")
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Clowder Test",
                "-c",
                "user.email=test@example.invalid",
            ])
            .args(args)
            .current_dir(path)
            .output()
            .unwrap(),
    )
    .trim()
    .to_owned()
}

fn project(root: &Path, name: &str) {
    let path = root.join(name);
    fs::create_dir(&path).unwrap();
    git(&path, &["init", "--initial-branch=main"]);
    fs::write(path.join("tracked.txt"), "original\n").unwrap();
    git(&path, &["add", "tracked.txt"]);
    git(&path, &["commit", "-m", "Initial"]);
}

#[test]
fn preserves_python_reference_output() {
    let fixture: Value =
        serde_json::from_str(include_str!("data/rust/output-compatibility.json")).unwrap();
    let directory = TempDir::new().unwrap();
    fs::write(
        directory.path().join("clowder.yml"),
        fixture["manifest"].as_str().unwrap(),
    )
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let args: Vec<_> = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        assert_eq!(
            success(clowder(directory.path(), &args)),
            case["stdout"].as_str().unwrap(),
            "arguments: {args:?}"
        );
    }
    assert_eq!(
        success(clowder(directory.path(), &["--ver"])),
        success(clowder(directory.path(), &["--version"]))
    );
    assert_eq!(
        success(clowder(directory.path(), &["herd", "--he"])),
        success(clowder(directory.path(), &["herd", "--help"]))
    );
}

#[test]
fn parallel_forall_is_concurrent_and_isolates_project_environments() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    fs::write(
        root.join("clowder.yml"),
        "name: local\ndefaults: {branch: main}\nclowder: [owner/alpha, owner/beta]\n",
    )
    .unwrap();
    project(root, "alpha");
    project(root, "beta");
    // Each process must see the other process start before it can complete.
    // A sequential implementation times out instead of passing by coincidence.
    let command = r#"printf '%s\n' "$PROJECT_NAME|$PROJECT_PATH|$PROJECT_REF" > identity; touch "$CLOWDER_PATH/$(basename "$PROJECT_PATH").ready"; n=0; while [ ! -f "$CLOWDER_PATH/alpha.ready" ] || [ ! -f "$CLOWDER_PATH/beta.ready" ]; do n=$((n+1)); [ "$n" -lt 100 ] || exit 77; sleep 0.1; done"#;
    success(clowder(root, &["forall", "-j2", command]));
    for name in ["alpha", "beta"] {
        let identity = fs::read_to_string(root.join(name).join("identity")).unwrap();
        assert_eq!(
            identity,
            format!(
                "owner/{name}|{}/{name}|refs/heads/main\n",
                root.canonicalize().unwrap().display()
            )
        );
    }
    assert_eq!(
        clowder(root, &["forall", "-j2", "exit 42"]).status.code(),
        Some(42)
    );
    success(clowder(root, &["forall", "-ij2", "exit 42"]));
}

#[test]
fn validates_every_project_before_mutating_any_branch() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    fs::write(
        root.join("clowder.yml"),
        "name: local\ndefaults: {branch: main}\nclowder: [owner/alpha, owner/beta]\n",
    )
    .unwrap();
    project(root, "alpha");
    project(root, "beta");
    fs::write(root.join("beta/tracked.txt"), "keep my changes\n").unwrap();
    assert!(!clowder(root, &["start", "feature"]).status.success());
    assert_eq!(
        git(&root.join("alpha"), &["branch", "--show-current"]),
        "main"
    );
    assert_eq!(
        fs::read_to_string(root.join("beta/tracked.txt")).unwrap(),
        "keep my changes\n"
    );
}

#[test]
fn tracking_start_checks_remote_access_before_creating_a_local_branch() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    fs::write(
        root.join("clowder.yml"),
        "name: local\ndefaults: {branch: main}\nclowder: [owner/alpha]\n",
    )
    .unwrap();
    project(root, "alpha");
    git(
        &root.join("alpha"),
        &[
            "remote",
            "add",
            "origin",
            root.join("missing.git").to_str().unwrap(),
        ],
    );
    assert!(
        !clowder(root, &["start", "--tracking", "feature"])
            .status
            .success()
    );
    assert_eq!(
        git(&root.join("alpha"), &["branch", "--show-current"]),
        "main"
    );
    assert_eq!(
        git(&root.join("alpha"), &["branch", "--list", "feature"]),
        ""
    );
}

#[test]
fn git_herd_alias_quotes_paths_and_forwards_options() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    let name = "-猫's $(touch escaped); project";
    fs::write(
        root.join("clowder.yml"),
        serde_json::json!({
            "name": "local",
            "defaults": {"branch": "main"},
            "clowder": [{"name": "owner/project", "path": name}]
        })
        .to_string(),
    )
    .unwrap();
    project(root, name);
    let path = root.join(name);
    let remote = root.join("remote.git");
    git(
        root,
        &[
            "clone",
            "--bare",
            path.to_str().unwrap(),
            remote.to_str().unwrap(),
        ],
    );
    git(
        &path,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    success(clowder(root, &["herd"]));
    let executable = Path::new(env!("CARGO_BIN_EXE_clowder"));
    let paths = std::iter::once(executable.parent().unwrap().to_path_buf())
        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap()))
        .collect::<Vec<_>>();
    let result = Command::new("git")
        .args(["herd", "--rebase"])
        .env("PATH", std::env::join_paths(paths).unwrap())
        .current_dir(&path)
        .output()
        .unwrap();
    assert!(success(result).contains("Pull with rebase"));
    assert!(!path.join("escaped").exists());
    assert_eq!(git(&path, &["branch", "--show-current"]), "main");
}

#[cfg(unix)]
#[test]
fn rejects_project_paths_that_resolve_to_the_same_repository() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    project(root, "actual");
    std::os::unix::fs::symlink("actual", root.join("alias")).unwrap();
    for suffix in ["", "/not-cloned-yet"] {
        fs::write(
            root.join("clowder.yml"),
            serde_json::json!({
                "name": "local", "defaults": {"branch": "main"},
                "clowder": [
                    {"name": "owner/one", "path": format!("actual{suffix}")},
                    {"name": "owner/two", "path": format!("alias{suffix}")}
                ]
            })
            .to_string(),
        )
        .unwrap();
        let result = clowder(root, &["forall", "-j2", "touch should-not-run"]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("Multiple projects with path"));
        assert!(!root.join("actual/should-not-run").exists());
    }
}

#[cfg(unix)]
#[test]
fn parent_components_are_resolved_after_symlinks() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    fs::create_dir_all(root.join("outside/child")).unwrap();
    fs::create_dir(root.join("workspace")).unwrap();
    std::os::unix::fs::symlink("../outside/child", root.join("workspace/alias")).unwrap();
    project(&root.join("outside"), "project");
    fs::write(
        root.join("workspace/clowder.yml"),
        "name: local\ndefaults: {branch: main}\nclowder:\n  - name: owner/project\n    path: alias/../project\n",
    ).unwrap();
    success(clowder(
        &root.join("workspace"),
        &["forall", "touch selected"],
    ));
    assert!(root.join("outside/project/selected").is_file());
    assert!(!root.join("workspace/project").exists());
}

#[test]
fn sequential_forall_preserves_standard_input_and_error() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    project(root, "alpha");
    fs::write(
        root.join("clowder.yml"),
        "name: local\ndefaults: {branch: main}\nclowder: [owner/alpha]\n",
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_clowder"))
        .args(["forall", "read value; printf '%s:%s\\n' \"$PROJECT_NAME\" \"$value\"; printf 'child stderr\\n' >&2"])
        .current_dir(root)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"from stdin\n")
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert_eq!(String::from_utf8_lossy(&result.stderr), "child stderr\n");
    assert!(success(result).contains("owner/alpha:from stdin\n"));
}
