use alloc::string::String;
use crate::syscall::*;

/// prints a string to the kernel console
pub fn kprint(string: &String) -> i64 {
    unsafe {
        let string = string.as_bytes();
        syscall!(KPRINT, string.as_ptr() as u64, string.len())
    }
}

