//! Owned process groups let cancellation reach Git helpers and shell children.
//! Signal handlers only set an atomic flag; cleanup runs in ordinary Rust code.
use crate::commands::ExitCode;
use anyhow::Result;
use nix::{
    sys::signal::{SigSet, SigmaskHow, Signal, killpg, pthread_sigmask},
    unistd::{Pid, getpgrp, tcgetpgrp, tcsetpgrp},
};
use std::{
    cell::Cell,
    fs::File,
    io::{IsTerminal, Read, Seek, SeekFrom},
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Child, Command, ExitStatus, Output, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

static CANCELLED: OnceLock<Arc<AtomicUsize>> = OnceLock::new();
thread_local! { static INTERACTIVE: Cell<bool> = const { Cell::new(true) }; }

pub fn init() -> Result<()> {
    let flag = Arc::new(AtomicUsize::new(0));
    for signal in [Signal::SIGINT, Signal::SIGTERM, Signal::SIGHUP] {
        signal_hook::flag::register_usize(signal as i32, Arc::clone(&flag), signal as usize)?;
    }
    let _ = CANCELLED.set(flag);
    Ok(())
}

pub fn exit_code() -> Option<i32> {
    CANCELLED
        .get()
        .map(|flag| flag.load(Ordering::Relaxed))
        .filter(|s| *s != 0)
        .map(|s| 128 + s as i32)
}

pub fn check() -> Result<()> {
    if let Some(code) = exit_code() {
        Err(ExitCode(code).into())
    } else {
        Ok(())
    }
}

pub fn interactive<T>(enabled: bool, run: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            INTERACTIVE.set(self.0);
        }
    }
    let _restore = Restore(INTERACTIVE.replace(enabled));
    run()
}

struct Terminal(File, Pid);
impl Terminal {
    fn give_to(group: Pid) -> Option<Self> {
        if !INTERACTIVE.get() || !std::io::stdin().is_terminal() {
            return None;
        }
        let tty = File::open("/dev/tty").ok()?;
        let parent = getpgrp();
        if tcgetpgrp(&tty).ok()? != parent {
            return None;
        }
        Self::set(&tty, group).ok()?;
        // A child can try reading between spawn and foreground handoff.
        let _ = killpg(group, Signal::SIGCONT);
        Some(Self(tty, parent))
    }

    fn set(tty: &File, group: Pid) -> nix::Result<()> {
        let mut blocked = SigSet::empty();
        blocked.add(Signal::SIGTTOU);
        let mut previous = SigSet::empty();
        pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&blocked), Some(&mut previous))?;
        let result = tcsetpgrp(tty, group);
        let restore = pthread_sigmask(SigmaskHow::SIG_SETMASK, Some(&previous), None);
        result.and(restore)
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = Self::set(&self.0, self.1);
    }
}

struct Running {
    child: Child,
    group: Pid,
    finished: bool,
    _terminal: Option<Terminal>,
}
impl Running {
    fn spawn(command: &mut Command) -> Result<Self> {
        check()?;
        let child = command.process_group(0).spawn()?;
        let group = Pid::from_raw(child.id() as i32);
        Ok(Self {
            child,
            group,
            finished: false,
            _terminal: Terminal::give_to(group),
        })
    }

    fn stop(&mut self, signal: Signal) {
        let _ = killpg(self.group, signal);
        // Keep the group leader unreaped during the grace period so its PID
        // cannot be reused. Kill stubborn descendants even if the shell exits.
        thread::sleep(Duration::from_millis(250));
        let _ = killpg(self.group, Signal::SIGKILL);
        let _ = self.child.wait();
        self.finished = true;
    }

    fn wait(mut self) -> Result<ExitStatus> {
        loop {
            if let Some(code) = exit_code() {
                self.stop(Signal::try_from(code - 128).unwrap_or(Signal::SIGTERM));
                return Err(ExitCode(code).into());
            }
            if let Some(status) = self.child.try_wait()? {
                if let Some(signal) = status.signal()
                    && [
                        Signal::SIGINT as i32,
                        Signal::SIGTERM as i32,
                        Signal::SIGHUP as i32,
                    ]
                    .contains(&signal)
                {
                    if let Some(flag) = CANCELLED.get() {
                        flag.store(signal as usize, Ordering::Relaxed);
                    }
                    self.stop(Signal::try_from(signal)?);
                    return Err(ExitCode(128 + signal).into());
                }
                self.finished = true;
                return Ok(status);
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        if !self.finished {
            let _ = killpg(self.group, Signal::SIGKILL);
            let _ = self.child.wait();
        }
    }
}

pub fn status(command: &mut Command) -> Result<ExitStatus> {
    Running::spawn(command)?.wait()
}

pub fn output(command: &mut Command) -> Result<Output> {
    // Anonymous files avoid full pipes and reader threads stuck on a helper
    // which inherits Git's descriptors. They are removed even on cancellation.
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    let status = status(command)?;
    let read = |file: &mut File| -> Result<Vec<u8>> {
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    Ok(Output {
        status,
        stdout: read(&mut stdout)?,
        stderr: read(&mut stderr)?,
    })
}
