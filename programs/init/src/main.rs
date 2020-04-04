#![no_std]
#![no_main]
#![feature(start)]

extern crate bstd;
use bstd::*;

#[start]
#[no_mangle]
fn _start() {
    unsafe {
        syscall!(0);
    }
    loop {}
}

