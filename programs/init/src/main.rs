#![no_std]
#![no_main]
#![feature(start)]

extern crate alloc;
extern crate bstd;

use bstd::kprint::kprint;
use alloc::string::*;

type Buffer = [[(u8, u8); 80]; 25];

#[start]
#[no_mangle]
fn _start() {
    let message = "Hello World from a process using syscalls\n".to_string();
    kprint(&message);
    loop {}
}

