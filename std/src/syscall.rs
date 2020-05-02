//! syscall module that allows for communication with the kernel
#![macro_use]

pub use dep::syscall::*;


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
    pub fn _syscall(_rdi: u64, _rsi: u64, _rdx: u64, _rcx: u64, _r8: u64, _r9: u64) -> i64;
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

