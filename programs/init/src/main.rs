#![no_std]
#![no_main]
#![feature(start)]

extern crate bstd;
use bstd::*;

type Buffer = [[(u8, u8); 80]; 25];

#[start]
#[no_mangle]
fn _start() {
    unsafe {
        let buffer = &mut *(0xb8000 as *mut Buffer);
        buffer[0][0] = (b'X' as u8, 0 << 4 | 15);
        for i in 0..10_000_000 {
        }
        syscall!(0);
        loop {}
    }
}

