use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use spin::*;

use x86_64::structures::paging::{PageTableFlags};

use dep::consts::*;

use crate::memory;
use crate::elf;

pub mod schedule;

static CURRENT_PID: AtomicU64 = AtomicU64::new(0);
static NEXT_PID: AtomicU64 = AtomicU64::new(1);
static PROCESSES: Once<Mutex<BTreeMap<u64, Process>>> = Once::new();
static PROCESSES_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn init() {
    {
        processes();
    }
    schedule::init();
}

pub fn processes() -> MutexGuard<'static, BTreeMap<u64, Process>> {
    PROCESSES.call_once(|| Mutex::new(BTreeMap::new())).lock()
}

pub unsafe fn update() {
    if PROCESSES_ACTIVE.load(Ordering::SeqCst) {
        let next_pid = schedule::next_turn();
        switch_process(next_pid);
    }
}

pub struct Registers {
    rsp: u64,
    cr3: u64,
}

pub struct FileDescriptors {
    next_fd: i64,
}

impl FileDescriptors {
    fn new() -> Self { 
        Self {
            next_fd: 0,
        } 
    }
}

pub struct Process {
    pub id: u64,
    pub name: Vec<u8>,
    pub regs: Registers,
    pub files: FileDescriptors,
}

impl Process {
    pub unsafe fn create(exec_path: String) -> u64 {
        let id = NEXT_PID.fetch_add(1, Ordering::SeqCst);
        let mut proc = Process {
            id,
            name: b"PROC".to_vec(),
            regs: Registers {
                rsp: USER_STACK_TOP,
                cr3: memory::new_table(id),
            },
            files: FileDescriptors::new(),
        };

        let old_table = memory::load_table(proc.regs.cr3);

        memory::map_range(
            USER_STACK_TOP - USER_STACK_SIZE, 
            USER_STACK_TOP, 
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
        ).expect("mapping user stack failed");

        memory::map_range(
            USER_HEAP_START,
            USER_HEAP_START + USER_HEAP_SIZE,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
        ).expect("mapping user heap failed");


        let entry_point = elf::load_elf(exec_path).expect("Failed to load ELF");

        proc.push_to_stack(entry_point); // rip
        for _ in 0..6 {
            // rbx rbp r12 r13 r14 r15
            proc.push_to_stack(0);
        }
        proc.push_to_stack(0x200); // rflags

        processes().insert(id, proc);
        memory::load_table(old_table);

        PROCESSES_ACTIVE.compare_and_swap(false, true, Ordering::SeqCst);

        id
    }

    pub unsafe fn push_to_stack(&mut self, value: u64) {
        self.regs.rsp -= core::mem::size_of::<u64>() as u64;
        *(self.regs.rsp as *mut u64) = value;
    }
}

use crate::println;
pub unsafe fn switch_process(next_pid: u64) {
    let mut processes = processes();
    let current_pid = CURRENT_PID.load(Ordering::SeqCst);

    use crate::serial_println;
    serial_println!("Switching from {} to {}", current_pid, next_pid);

    if current_pid == next_pid {
        return;
    }

    let next_process = match processes.get(&next_pid) {
        Some(proc) => proc,
        None => return,
    };

    let next_rsp = next_process.regs.rsp;
    let next_cr3 = next_process.regs.cr3;

    let mut garbage_rsp: u64 = 0;
    let current_rsp =
        match processes.get_mut(&current_pid) {
            Some(proc) => (&mut proc.regs.rsp) as *mut u64,
            None => (&mut garbage_rsp) as *mut u64,
        };

    CURRENT_PID.store(next_pid, Ordering::SeqCst);

    drop(processes); // avoid poisoning the processes mutex when switching to userspace

    switch_context(current_rsp, next_rsp, next_cr3);
}

extern "C" {
    fn switch_context(current_rsp: *mut u64, next_rsp: u64, next_cr3: u64);
}

