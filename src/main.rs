#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rust_os::{print, println};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    rust_os::init();

    #[cfg(test)]
    test_main();

    //println!("Hello World{}", "!");

    let mut x = 0;
    for _ in 0..100 {
        print!("-");
        x += 1;
        for _ in 0..100000 {}
    }
    rust_os::hlt_loop()
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    rust_os::hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_os::test_panic_handler(info)
}
