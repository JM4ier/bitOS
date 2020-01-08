//! syscall module that allows for communication with the kernel

/// print to kernel console
pub const PRINT: u64 = 0x10;

/// open file or directory
pub const OPEN: u64 = 0x20;

/// create file
pub const CREATE: u64 = 0x21;

/// read file
pub const READ: u64 = 0x22;

/// write file
pub const WRITE: u64 = 0x23;

/// remove file
pub const REMOVE: u64 = 0x24;

/// read directory content
pub const READDIR: u64 = 0x25;

/// create directory
pub const MKDIR: u64 = 0x25;

/// remove directory
pub const RMDIR: u64 = 0x26;




// taken from https://github.com/kryo4096/RostOS/blob/master/rost_std/src/syscall.rs
global_asm! (
"
.global _syscall

_syscall:
    int $0x80
    ret
"
);


extern "C" {
    pub fn _syscall(_rdi: u64, _rsi: u64, _rdx: u64, _rcx: u64, _r8: u64, _r9: u64) -> u64;
}

#[macro_export]
macro_rules! syscall {
    ($rdi:expr) => {
        crate::syscall::_syscall($rdi as _, 0, 0, 0, 0, 0)
    };
    ($rdi:expr, $rsi:expr) => {
        crate::syscall::_syscall($rdi as _, $rsi as _, 0, 0, 0, 0)
    };
    ($rdi:expr, $rsi:expr, $rdx:expr) => {
        crate::syscall::_syscall($rdi as _, $rsi as _, $rdx as _, 0, 0, 0)
    };
    ($rdi:expr, $rsi:expr, $rdx:expr, $rcx:expr) => {
        crate::syscall::_syscall($rdi as _, $rsi as _, $rdx as _, $rcx as _, 0, 0)
    };
    ($rdi:expr, $rsi:expr, $rdx:expr, $rcx:expr, $r8:expr) => {
        crate::syscall::_syscall($rdi as _, $rsi as _, $rdx as _, $rcx as _, $r8 as _, 0)
    };
    ($rdi:expr, $rsi:expr, $rdx:expr, $rcx:expr, $r8:expr, $r9:expr) => {
        crate::syscall::_syscall(
            $rdi as _, $rsi as _, $rdx as _, $rcx as _, $r8 as _, $r9 as _,
        )
    };
}

