extern crate alloc;

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
pub unsafe extern "C" fn __syscall(syscall_number: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> i64 {
    match syscall_number {
        KPRINT => kprint(arg0, arg1),
        OPEN => open(arg0, arg1, arg2),
        CLOSE => close(arg0),
        READ => read(arg0, arg1, arg2),
        WRITE => write(arg0, arg1, arg2),
        _ => {
            println!("unknown syscall {}", syscall_number);
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

use dep::fs::{*, error::*};
use crate::files::*;
use core::ptr::*;
use alloc::string::*;

/// open a file for the current process and return an integer representing the file
unsafe fn open(path: u64, path_len: u64, flags: u64) -> i64 {
    let path = &*slice_from_raw_parts::<u8>(path as *const u8, path_len as _);
    if let Ok(path) = String::from_utf8(path.to_vec()) {
        let path = Path::new(path);
        if let Some(path) = path {
            let result = match flags {
                0 => fs().open_read(path),
                1 => fs().open_write(path),
                _ => return OTHER,
            };
            return map_err(result);
        }
    }
    return OTHER;
}

/// close a file given by the file descriptor
unsafe fn close(fd: u64) -> i64 {
    0 // TODO
}

/// read an opened file given by the file descriptor
unsafe fn read(fd: u64, bytes: u64, bytes_len: u64) -> i64 {
    let bytes = &mut *slice_from_raw_parts_mut::<u8>(bytes as *mut u8, bytes_len as _);
    map_err(fs().read(fd as _, bytes).map(|x| x as i64))
}

/// write to an opened file 
unsafe fn write(fd: u64, bytes: u64, bytes_len: u64) -> i64 {
    let bytes = &*slice_from_raw_parts::<u8>(bytes as *const u8, bytes_len as _);
    map_err(fs().write(fd as _, bytes).map(|_| 0))
}

