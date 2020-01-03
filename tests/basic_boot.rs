#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bit_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    bit_os::test_panic_handler(info)
}

use bit_os::{println, serial_print, serial_println};

#[test_case]
fn test_println() {
    serial_print!("test_println... ");
    println!("test_println asdf");
    serial_println!("[ok]");
}
