use std::collections::{BTreeMap, VecDeque};
use std::thread;
use std::time::Duration;

use crate::InitConfig;
use crate::service::{Service, ServiceStart};
use crate::unit::{Unit, UnitId, UnitKind, UnitStore};

pub struct Scheduler {
    pending: VecDeque<Job>,
    supervised: BTreeMap<usize, SupervisedService>,
}

struct SupervisedService {
    service: Service,
    envs: BTreeMap<String, std::ffi::OsString>,
    restarts_remaining: u8,
}

struct Job {
    unit: UnitId,
    kind: JobKind,
}

enum JobKind {
    Start,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            pending: VecDeque::new(),
            supervised: BTreeMap::new(),
        }
    }

    pub fn schedule_start_and_report_errors(
        &mut self,
        unit_store: &mut UnitStore,
        unit_id: UnitId,
    ) {
        let mut errors = vec![];
        self.schedule_start(unit_store, unit_id, &mut errors);
        for error in errors {
            eprintln!("init: {error}");
        }
    }

    pub fn schedule_start(
        &mut self,
        unit_store: &mut UnitStore,
        unit_id: UnitId,
        errors: &mut Vec<String>,
    ) {
        let loaded_units = unit_store.load_units(unit_id.clone(), errors);
        for unit_id in loaded_units {
            if !unit_store.unit(&unit_id).conditions_met() {
                continue;
            }

            self.pending.push_back(Job {
                unit: unit_id,
                kind: JobKind::Start,
            });
        }
    }

    pub fn step(&mut self, unit_store: &mut UnitStore, init_config: &mut InitConfig) {
        'a: loop {
            let Some(job) = self.pending.pop_front() else {
                return;
            };

            match job.kind {
                JobKind::Start => {
                    let unit = unit_store.unit(&job.unit);

                    for dep in &unit.info.requires_weak {
                        for pending_job in &self.pending {
                            if &pending_job.unit == dep {
                                self.pending.push_back(job);
                                continue 'a;
                            }
                        }
                    }

                    if let Some((pid, service)) = run(unit, init_config) {
                        self.supervised.insert(
                            pid,
                            SupervisedService {
                                restarts_remaining: service.max_restarts.min(10),
                                service,
                                envs: init_config.envs.clone(),
                            },
                        );
                    }
                }
            }
        }
    }

    /// Called only by PID 1's central waitpid loop after the child is reaped.
    /// This keeps one authoritative reaper and avoids racing Child::wait callers.
    pub fn child_exited(&mut self, pid: usize, status: usize) {
        let Some(mut supervised) = self.supervised.remove(&pid) else {
            return;
        };

        // A zero wait status is a normal successful exit. Restart only failures.
        if status == 0 {
            return;
        }

        if supervised.restarts_remaining == 0 {
            eprintln!(
                "init: supervised service {:?} exited with status {status:#x}; restart budget exhausted",
                supervised.service.cmd,
            );
            return;
        }

        supervised.restarts_remaining -= 1;
        let delay = Duration::from_millis(
            supervised.service.restart_delay_ms.min(30_000) as u64,
        );
        if !delay.is_zero() {
            thread::sleep(delay);
        }

        match supervised.service.spawn(&supervised.envs) {
            ServiceStart::Running(new_pid) => {
                self.supervised.insert(new_pid, supervised);
            }
            ServiceStart::Completed => {}
            ServiceStart::Failed => eprintln!(
                "init: supervised service {:?} could not be restarted",
                supervised.service.cmd,
            ),
        }
    }
}

fn run(unit: &Unit, config: &mut InitConfig) -> Option<(usize, Service)> {
    match &unit.kind {
        UnitKind::LegacyScript { script } => {
            for cmd in script.clone() {
                if config.log_debug {
                    eprintln!("init: running: {cmd:?}");
                }
                cmd.run(config);
            }
        }
        UnitKind::Service { service } => {
            if config.skip_cmd.contains(&service.cmd) {
                eprintln!("Skipping '{} {}'", service.cmd, service.args.join(" "));
                return None;
            }
            if config.log_debug {
                eprintln!(
                    "Starting {} ({})",
                    unit.info.description.as_ref().unwrap_or(&unit.id.0),
                    service.cmd,
                );
            }
            match service.spawn(&config.envs) {
                ServiceStart::Running(pid)
                    if service.restart_on_failure && service.max_restarts > 0 =>
                {
                    return Some((pid, service.clone()));
                }
                ServiceStart::Running(_) | ServiceStart::Completed => {}
                ServiceStart::Failed => eprintln!(
                    "init: service {} exhausted its startup attempts; dependent units may continue only through weak dependencies",
                    unit.info.description.as_ref().unwrap_or(&unit.id.0),
                ),
            }
        }
        UnitKind::Target {} => {
            if config.log_debug {
                eprintln!(
                    "Reached target {}",
                    unit.info.description.as_ref().unwrap_or(&unit.id.0),
                );
            }
        }
    }
    None
}
