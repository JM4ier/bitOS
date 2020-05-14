#![no_std]
///! This is a small library of things that both the kernel and userspace need

extern crate alloc;

pub mod syscall;
pub mod consts;
pub mod fs;

