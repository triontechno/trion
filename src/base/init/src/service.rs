use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Read;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::{env, io, thread, time::Duration};

use serde::Deserialize;

use crate::script::subst_env;

// Keep restart work bounded even if a unit file is misconfigured.
const MAX_RESTART_ATTEMPTS: u8 = 10;
const MAX_RESTART_DELAY_MS: u32 = 30_000;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub envs: BTreeMap<String, String>,
    #[serde(default)]
    pub inherit_envs: BTreeSet<String>,
    #[serde(rename = "type")]
    pub type_: ServiceType,
    #[serde(default)]
    pub priority: Option<i32>,
    /// Retry only failures detected before startup/readiness completes.
    #[serde(default)]
    pub restart_on_failure: bool,
    /// Number of additional startup attempts, capped at MAX_RESTART_ATTEMPTS.
    #[serde(default)]
    pub max_restarts: u8,
    /// Delay between attempts, capped at MAX_RESTART_DELAY_MS.
    #[serde(default)]
    pub restart_delay_ms: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    #[default]
    Notify,
    Scheme(String),
    Oneshot,
    OneshotAsync,
}

impl Service {
    /// Start a service with bounded retries and, for long-running services,
    /// retain a monitor that can restart it after an unsuccessful exit.
    pub fn spawn(&self, base_envs: &BTreeMap<String, OsString>) -> bool {
        let retries = if self.restart_on_failure {
            self.max_restarts.min(MAX_RESTART_ATTEMPTS)
        } else {
            0
        };
        let delay = Duration::from_millis(
            self.restart_delay_ms.min(MAX_RESTART_DELAY_MS) as u64,
        );

        let mut remaining = retries;
        loop {
            match self.spawn_once(base_envs) {
                Ok(SpawnResult::Running(child)) => {
                    if remaining > 0 {
                        let service = self.clone();
                        let envs = base_envs.clone();
                        thread::spawn(move || {
                            supervise(service, envs, child, remaining, delay);
                        });
                    }
                    return true;
                }
                Ok(SpawnResult::Completed(true)) => return true,
                Ok(SpawnResult::Completed(false)) => {
                    eprintln!("init: service {:?} exited unsuccessfully during startup", self.cmd);
                }
                Err(err) => {
                    eprintln!("init: service {:?} startup failed: {err}", self.cmd);
                }
            }

            if remaining == 0 {
                return false;
            }
            remaining -= 1;
            if !delay.is_zero() {
                thread::sleep(delay);
            }
        }
    }

    fn spawn_once(&self, base_envs: &BTreeMap<String, OsString>) -> io::Result<SpawnResult> {
        let mut command = Command::new(&self.cmd);
        command.args(self.args.iter().map(|arg| subst_env(arg)));
        command.env_clear();
        for env in &self.inherit_envs {
            if let Some(value) = env::var_os(env) {
                command.env(env, value);
            }
        }
        command.envs(base_envs).envs(&self.envs);

        let (mut read_pipe, write_pipe) = io::pipe()?;
        unsafe { pass_fd(&mut command, "INIT_NOTIFY", write_pipe.as_raw_fd()) };

        if let Some(priority) = self.priority {
            unsafe {
                command.pre_exec(move || {
                    if libc::setpriority(libc::PRIO_PROCESS, 0, priority) == -1 {
                        Err(io::Error::last_os_error())
                    } else {
                        Ok(())
                    }
                });
            }
        }

        let mut child = command.spawn()?;
        drop(write_pipe);

        let startup = match &self.type_ {
            ServiceType::Notify => {
                let mut ready = [0];
                match read_pipe.read_exact(&mut ready) {
                    Ok(()) => Ok(true),
                    Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
                    Err(err) => Err(err),
                }
            }
            ServiceType::Scheme(scheme) => {
                let mut new_fd = usize::MAX;
                loop {
                    match libredox::call::call_ro(
                        read_pipe.as_raw_fd() as usize,
                        unsafe { plain::as_mut_bytes(&mut new_fd) },
                        syscall::CallFlags::FD | syscall::CallFlags::FD_UPPER,
                        &[],
                    ) {
                        Err(err) if err.is_interrupt() => continue,
                        Ok(0) => break Ok(false),
                        Ok(1) => {
                            let namespace_fd = match libredox::call::getns() {
                                Ok(fd) => fd,
                                Err(err) => break Err(io::Error::other(format!(
                                    "failed to get current namespace: {err}"
                                ))),
                            };
                            match libredox::call::register_scheme_to_ns(
                                namespace_fd,
                                scheme,
                                new_fd,
                            ) {
                                Ok(()) => break Ok(true),
                                Err(err) => break Err(io::Error::other(format!(
                                    "failed to register scheme {scheme}: {err}"
                                ))),
                            }
                        }
                        Ok(n) => break Err(io::Error::other(format!(
                            "service returned {n} descriptors; expected exactly one"
                        ))),
                        Err(err) => break Err(io::Error::other(format!(
                            "failed waiting for scheme readiness: {err}"
                        ))),
                    }
                }
            }
            ServiceType::Oneshot => {
                drop(read_pipe);
                let status = child.wait()?;
                return Ok(SpawnResult::Completed(status.success()));
            }
            ServiceType::OneshotAsync => Ok(true),
        };

        match startup {
            Ok(true) => Ok(SpawnResult::Running(child)),
            Ok(false) => {
                let _ = child.kill();
                let _ = child.wait();
                Ok(SpawnResult::Completed(false))
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(err)
            }
        }
    }
}

enum SpawnResult {
    Running(std::process::Child),
    Completed(bool),
}

fn supervise(
    service: Service,
    base_envs: BTreeMap<String, OsString>,
    mut child: std::process::Child,
    mut remaining: u8,
    delay: Duration,
) {
    loop {
        match child.wait() {
            Ok(status) if status.success() => return,
            Ok(status) => eprintln!(
                "init: supervised service {:?} exited with {status}",
                service.cmd,
            ),
            Err(err) => eprintln!(
                "init: failed waiting for supervised service {:?}: {err}",
                service.cmd,
            ),
        }

        if remaining == 0 {
            eprintln!(
                "init: service {:?} exhausted its runtime restart budget",
                service.cmd,
            );
            return;
        }
        remaining -= 1;
        if !delay.is_zero() {
            thread::sleep(delay);
        }

        loop {
            match service.spawn_once(&base_envs) {
                Ok(SpawnResult::Running(next_child)) => {
                    child = next_child;
                    break;
                }
                Ok(SpawnResult::Completed(true)) => return,
                Ok(SpawnResult::Completed(false)) => eprintln!(
                    "init: restarted service {:?} exited unsuccessfully",
                    service.cmd,
                ),
                Err(err) => eprintln!(
                    "init: failed to restart service {:?}: {err}",
                    service.cmd,
                ),
            }

            if remaining == 0 {
                eprintln!(
                    "init: service {:?} exhausted its runtime restart budget",
                    service.cmd,
                );
                return;
            }
            remaining -= 1;
            if !delay.is_zero() {
                thread::sleep(delay);
            }
        }
    }
}

unsafe fn pass_fd(cmd: &mut Command, env: &str, fd: RawFd) {
    cmd.env(env, format!("{}", fd));
    unsafe {
        cmd.pre_exec(move || {
            // Pass notify pipe to child
            if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}
