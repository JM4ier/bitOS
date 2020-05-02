use core::sync::atomic::Ordering;
use spin::*;

static SCHEDULER: Once<Mutex<Scheduler>> = Once::new();

pub fn init() {
    scheduler();
}

fn scheduler() -> MutexGuard<'static, Scheduler> {
    SCHEDULER.call_once(|| Mutex::new(Scheduler::new())).lock()
}

pub fn next_turn() -> u64 {
    let mut scheduler = scheduler();
    let processes = super::processes();
    let max_pid = super::NEXT_PID.load(Ordering::Acquire);
    let mut min_pid = u64::MAX;
    let mut next_pid = None;

    for pid in 0..max_pid {
        if !processes.contains_key(&pid) {
            continue;
        }

        min_pid = min_pid.min(pid);
        if next_pid == None && pid > scheduler.current_process {
            next_pid = Some(pid);
        }
    }

    scheduler.current_process = 
        match next_pid {
            Some(pid) => pid,
            None => min_pid,
        };

    scheduler.current_process
}

struct Scheduler {
    current_process: u64,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            current_process: 0,
        }
    }
}

