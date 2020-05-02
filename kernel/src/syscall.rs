use x86_64::structures::paging::PageTableFlags;
use dep::syscall::*;
use dep::consts::*;

use crate::{print, println, serial_println};
use crate::memory::map_range;

pub fn init_syscall_stack() {
    let syscall_stack_start = KERNEL_SYSCALL_STACK_TOP - KERNEL_SYSCALL_STACK_SIZE + 1;
    let syscall_stack_end = KERNEL_SYSCALL_STACK_TOP;
    map_range(
        syscall_stack_start,
        syscall_stack_end,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE
    ).unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn __syscall(rdi: u64, rsi: u64, rdx: u64, rcx: u64, r8: u64, r9: u64) -> i64 {
    match rdi {
        KPRINT => kprint(rsi, rdx),
        _ => {
            println!("unknown syscall {}", rdi);
            0
        }
    }
}

unsafe fn kprint(ptr: u64, len: u64) -> i64 {
    let slice = core::slice::from_raw_parts(ptr as _, len as usize);
    let s = core::str::from_utf8_unchecked(slice);
    print!("{}", s);
    0
}

