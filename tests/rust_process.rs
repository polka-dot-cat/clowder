//! Failure tests own every process group and use local repositories only.
#![cfg(unix)]
use std::{
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn workspace(names: &[&str]) -> TempDir {
    let root = TempDir::new().unwrap();
    let projects = names
        .iter()
        .map(|n| format!("owner/{n}"))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.path().join("clowder.yml"),
        format!("name: local\ndefaults: {{branch: main}}\nclowder: [{projects}]\n"),
    )
    .unwrap();
    for name in names {
        let path = root.path().join(name);
        fs::create_dir(&path).unwrap();
        git(&path, &["init", "--initial-branch=main"]);
        fs::write(path.join("tracked"), "original\n").unwrap();
        git(&path, &["add", "tracked"]);
        git(&path, &["commit", "-m", "Initial"]);
    }
    root
}

fn signal(pid: i32, name: &str) {
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };
    let signal = match name {
        "INT" => Signal::SIGINT,
        "TERM" => Signal::SIGTERM,
        "KILL" => Signal::SIGKILL,
        _ => unreachable!(),
    };
    let _ = kill(Pid::from_raw(pid), signal);
}

struct Running {
    child: Child,
    descendants: Vec<i32>,
}
impl Drop for Running {
    fn drop(&mut self) {
        // The test spawned Clowder into its own group, never the test runner's.
        signal(-(self.child.id() as i32), "KILL");
        for pid in &self.descendants {
            signal(*pid, "KILL");
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for child process"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
fn alive(pid: i32) -> bool {
    let output = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    let state = String::from_utf8_lossy(&output.stdout);
    !state.trim().is_empty() && !state.trim().starts_with('Z')
}

#[test]
fn interruption_stops_children_pending_projects_and_cleans_spools() {
    for (sig, code, jobs) in [
        ("INT", 130, "-j2"),
        ("TERM", 143, "-j2"),
        ("INT", 130, "-j1"),
    ] {
        let root = workspace(&["alpha", "beta", "gamma", "delta"]);
        let spools = root.path().join("spools");
        fs::create_dir(&spools).unwrap();
        // The grandchild deliberately ignores polite termination. Clowder must
        // escalate, and --ignore-errors must not turn cancellation into success.
        let script = r#"echo $$ > parent.pid; sh -c 'trap "" INT TERM; echo $$ > child.pid; while :; do sleep 1; done' & wait"#;
        let child = Command::new(env!("CARGO_BIN_EXE_clowder"))
            .args(["forall", "--ignore-errors", jobs, script])
            .current_dir(root.path())
            .env("TMPDIR", &spools)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap();
        let mut running = Running {
            child,
            descendants: Vec::new(),
        };
        let count = if jobs == "-j1" { 1 } else { 2 };
        until(|| {
            ["alpha", "beta", "gamma", "delta"]
                .iter()
                .filter(|n| root.path().join(n).join("child.pid").exists())
                .count()
                == count
        });
        for name in ["alpha", "beta", "gamma", "delta"] {
            for file in ["parent.pid", "child.pid"] {
                if let Ok(pid) = fs::read_to_string(root.path().join(name).join(file)) {
                    running.descendants.push(pid.trim().parse().unwrap());
                }
            }
        }
        signal(running.child.id() as i32, sig);
        let mut status = None;
        until(|| {
            status = running.child.try_wait().unwrap();
            status.is_some()
        });
        assert_eq!(status.unwrap().code(), Some(code));
        until(|| running.descendants.iter().all(|p| !alive(*p)));
        assert_eq!(
            ["alpha", "beta", "gamma", "delta"]
                .iter()
                .filter(|n| root.path().join(n).join("parent.pid").exists())
                .count(),
            count
        );
        assert_eq!(fs::read_dir(&spools).unwrap().count(), 0);
    }
}

#[test]
fn large_parallel_output_is_complete_ordered_and_removed_after_failure() {
    let root = workspace(&["alpha", "beta"]);
    let spools = root.path().join("spools");
    fs::create_dir(&spools).unwrap();
    let block = vec![b'x'; 2 * 1024 * 1024];
    fs::write(root.path().join("block"), &block).unwrap();
    for exit in [0, 42] {
        let command = format!(
            "printf 'BEGIN-%s\\n' \"$PROJECT_NAME\"; cat ../block; cat ../block >&2; printf '\\nEND-%s\\n' \"$PROJECT_NAME\"; exit {exit}"
        );
        let output = Command::new(env!("CARGO_BIN_EXE_clowder"))
            .args(["forall", "-j2", &command])
            .current_dir(root.path())
            .env("TMPDIR", &spools)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(exit));
        let text = String::from_utf8(output.stdout).unwrap();
        let payload = "x".repeat(block.len() * 2);
        let alpha = format!("BEGIN-owner/alpha\n{payload}\nEND-owner/alpha\n");
        let beta = format!("BEGIN-owner/beta\n{payload}\nEND-owner/beta\n");
        assert!(text.contains(&alpha) && text.contains(&beta));
        assert!(text.find(&alpha) < text.find(&beta));
        assert_eq!(fs::read_dir(&spools).unwrap().count(), 0);
    }
}

#[test]
fn failed_fetch_preserves_branch_commit_and_worktree() {
    let root = workspace(&["alpha"]);
    let path = root.path().join("alpha");
    git(
        &path,
        &["remote", "add", "origin", "https://example.invalid/alpha"],
    );
    git(&path, &["checkout", "-b", "topic"]);
    let head = git(&path, &["rev-parse", "HEAD"]);
    let shim = root.path().join("bin");
    fs::create_dir(&shim).unwrap();
    let real_git = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    fs::write(shim.join("git"), "#!/bin/sh\nif [ \"$1\" = fetch ]; then echo simulated-fetch-failure >&2; exit 42; fi\nexec \"$REAL_GIT\" \"$@\"\n").unwrap();
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_clowder"))
        .arg("herd")
        .current_dir(root.path())
        .env(
            "REAL_GIT",
            String::from_utf8(real_git.stdout).unwrap().trim(),
        )
        .env(
            "PATH",
            format!("{}:{}", shim.display(), std::env::var("PATH").unwrap()),
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(42));
    assert!(String::from_utf8_lossy(&output.stdout).contains("simulated-fetch-failure"));
    assert_eq!(git(&path, &["symbolic-ref", "--short", "HEAD"]), "topic");
    assert_eq!(git(&path, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        fs::read_to_string(path.join("tracked")).unwrap(),
        "original\n"
    );
    assert!(git(&path, &["status", "--porcelain"]).is_empty());
}

#[test]
fn sequential_forall_keeps_foreground_terminal_input() {
    let root = workspace(&["alpha"]);
    let output = fs::File::create(root.path().join("terminal.log")).unwrap();
    let executable = env!("CARGO_BIN_EXE_clowder");
    let script = "touch terminal.ready; read value; printf 'received:%s\\n' \"$value\"";
    let mut terminal = Command::new("script");
    #[cfg(target_os = "macos")]
    terminal.args(["-q", "/dev/null", executable, "forall", script]);
    #[cfg(not(target_os = "macos"))]
    {
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        terminal.args([
            "-q",
            "-e",
            "-c",
            &format!("{} forall {}", quote(executable), quote(script)),
            "/dev/null",
        ]);
    }
    let child = terminal
        .current_dir(root.path())
        .stdin(Stdio::piped())
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .process_group(0)
        .spawn()
        .unwrap();
    let mut running = Running {
        child,
        descendants: Vec::new(),
    };
    until(|| root.path().join("alpha/terminal.ready").exists());
    use std::io::Write;
    running
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello terminal\n")
        .unwrap();
    let mut status = None;
    until(|| {
        status = running.child.try_wait().unwrap();
        status.is_some()
    });
    assert!(status.unwrap().success());
    assert!(
        fs::read_to_string(root.path().join("terminal.log"))
            .unwrap()
            .contains("received:hello terminal")
    );
}

#[test]
fn cancellation_also_stops_captured_git_commands() {
    let root = workspace(&["alpha"]);
    let path = root.path().join("alpha");
    git(
        &path,
        &["remote", "add", "origin", "https://example.invalid/alpha"],
    );
    let shim = root.path().join("bin");
    fs::create_dir(&shim).unwrap();
    let real_git = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    fs::write(shim.join("git"), "#!/bin/sh\nif [ \"$1\" = fetch ]; then echo $$ > git.pid; exec sleep 60; fi\nexec \"$REAL_GIT\" \"$@\"\n").unwrap();
    fs::set_permissions(shim.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_clowder"))
        .arg("herd")
        .current_dir(root.path())
        .env(
            "REAL_GIT",
            String::from_utf8(real_git.stdout).unwrap().trim(),
        )
        .env(
            "PATH",
            format!("{}:{}", shim.display(), std::env::var("PATH").unwrap()),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .unwrap();
    let mut running = Running {
        child,
        descendants: Vec::new(),
    };
    until(|| path.join("git.pid").exists());
    running.descendants.push(
        fs::read_to_string(path.join("git.pid"))
            .unwrap()
            .trim()
            .parse()
            .unwrap(),
    );
    signal(running.child.id() as i32, "TERM");
    let mut status = None;
    until(|| {
        status = running.child.try_wait().unwrap();
        status.is_some()
    });
    assert_eq!(status.unwrap().code(), Some(143));
    until(|| running.descendants.iter().all(|pid| !alive(*pid)));
}

#[test]
fn terminal_ctrl_c_cancels_even_when_forall_ignores_command_errors() {
    use std::io::Write;
    let root = workspace(&["alpha", "beta"]);
    let output = fs::File::create(root.path().join("terminal.log")).unwrap();
    let executable = env!("CARGO_BIN_EXE_clowder");
    let script = "echo $PPID > ../clowder.pid; echo $$ > ../shell.pid; touch terminal.ready; sleep 60; touch after-sleep";
    let mut terminal = Command::new("script");
    #[cfg(target_os = "macos")]
    terminal.args([
        "-q",
        "/dev/null",
        executable,
        "forall",
        "--ignore-errors",
        script,
    ]);
    #[cfg(not(target_os = "macos"))]
    {
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        terminal.args([
            "-q",
            "-e",
            "-c",
            &format!(
                "{} forall --ignore-errors {}",
                quote(executable),
                quote(script)
            ),
            "/dev/null",
        ]);
    }
    let child = terminal
        .current_dir(root.path())
        .stdin(Stdio::piped())
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .process_group(0)
        .spawn()
        .unwrap();
    let mut running = Running {
        child,
        descendants: Vec::new(),
    };
    until(|| root.path().join("alpha/terminal.ready").exists());
    for file in ["clowder.pid", "shell.pid"] {
        let pid: i32 = fs::read_to_string(root.path().join(file))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        running.descendants.extend([pid, -pid]);
    }
    running
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&[3])
        .unwrap();
    let mut status = None;
    until(|| {
        status = running.child.try_wait().unwrap();
        status.is_some()
    });
    assert_eq!(status.unwrap().code(), Some(130));
    assert!(!root.path().join("beta/terminal.ready").exists());
    assert!(!root.path().join("alpha/after-sleep").exists());
}

#[test]
fn terminal_suspend_and_resume_preserve_foreground_input() {
    #[cfg(target_os = "macos")]
    let shells = ["/bin/sh"];
    #[cfg(not(target_os = "macos"))]
    let shells = ["/bin/sh", "/bin/bash"];
    for shell in shells {
        check_terminal_suspend_and_resume(shell);
    }
}

fn check_terminal_suspend_and_resume(shell: &str) {
    use std::io::Write;
    let root = workspace(&["alpha"]);
    let output = fs::File::create(root.path().join("terminal.log")).unwrap();
    let executable = env!("CARGO_BIN_EXE_clowder");
    let script = "echo $PPID > ../clowder.pid; echo $$ > ../shell.pid; touch terminal.ready; read value; printf 'resumed:%s\\n' \"$value\"";
    let mut terminal = Command::new("script");
    terminal.env("SHELL", shell);
    #[cfg(target_os = "macos")]
    terminal.args([
        "-q",
        "/dev/null",
        executable,
        "forall",
        "--ignore-errors",
        script,
    ]);
    #[cfg(not(target_os = "macos"))]
    {
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        terminal.args([
            "-q",
            "-e",
            "-c",
            &format!(
                "{} forall --ignore-errors {}",
                quote(executable),
                quote(script)
            ),
            "/dev/null",
        ]);
    }
    let child = terminal
        .current_dir(root.path())
        .stdin(Stdio::piped())
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .process_group(0)
        .spawn()
        .unwrap();
    let mut running = Running {
        child,
        descendants: Vec::new(),
    };
    until(|| root.path().join("alpha/terminal.ready").exists());
    for file in ["clowder.pid", "shell.pid"] {
        let pid: i32 = fs::read_to_string(root.path().join(file))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        running.descendants.extend([pid, -pid]);
    }
    running
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&[26])
        .unwrap();
    let clowder_pid = running.descendants[0];
    until(|| {
        let output = Command::new("ps")
            .args(["-o", "stat=", "-p", &clowder_pid.to_string()])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .starts_with('T')
    });
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(clowder_pid),
        nix::sys::signal::Signal::SIGCONT,
    );
    running
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello resume\n")
        .unwrap();
    let mut status = None;
    until(|| {
        status = running.child.try_wait().unwrap();
        if status.is_none() {
            // util-linux script suspends its relay when its direct child stops.
            // Bash can exec Clowder, whereas dash keeps an intermediate shell.
            // Resume the relay too, including a stop observed after our SIGCONT.
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(running.child.id() as i32),
                nix::sys::signal::Signal::SIGCONT,
            );
        }
        status.is_some()
    });
    assert_eq!(status.unwrap().code(), Some(0));
    assert!(
        fs::read_to_string(root.path().join("terminal.log"))
            .unwrap()
            .contains("resumed:hello resume")
    );
}
