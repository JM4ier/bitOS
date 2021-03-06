#![feature(asm, global_asm, naked_functions, alloc_error_handler)]
#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use core::alloc::Layout;

pub mod syscall;
pub mod kprint;
pub mod fs;
mod allocator;

#[no_mangle]
#[panic_handler]
pub fn panic(_panic_info: &PanicInfo) -> ! {
    // TODO
    loop {}
}

#[no_mangle]
#[alloc_error_handler]
pub fn alloc_error(_layout: Layout) -> ! {
    // TODO
    loop {}
}

