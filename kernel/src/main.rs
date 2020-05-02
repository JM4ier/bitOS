#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(global_asm)]
#![test_runner(bit_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use dep::consts::*;
use bit_os::{print, println, serial_println, memory, vga_buffer, vga_buffer::*, files, elf, syscall, process::{self, *}};
use bootloader::{BootInfo, entry_point};
use lazy_static::*;

extern crate alloc;
use alloc::string::String;

lazy_static! {
    static ref VGA_COLOR: ColorCode = ColorCode::new(Color::White, Color::Black);
}

entry_point!(kernel_main);
pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    set_color(*VGA_COLOR);

    println!("started boot sequence\n");
    println!("loading kernel features:");

    load_feature(|| {bit_os::init();}, "interrupts");
    load_feature(|| {
        // initializing kernel heap
        memory::init_boot_info(boot_info);
        memory::init_allocator();
        memory::heap::init_heap().expect("Heap initialization failed");
    }, "kernel heap");
    load_feature(files::init, "file system");
    load_feature(syscall::init_syscall_stack, "syscall stack");
    load_feature(process::init, "processes");

    kernel_start_message();

    files::message();

    unsafe {
        serial_println!("kernel syscall stack top at {:#x}", KERNEL_SYSCALL_STACK_TOP);
        serial_println!("memmap before process");
        memory::print_virt_memory_map();
        let init = Process::create(String::from("/bin/init"));
        serial_println!("memmap after process");
        memory::print_virt_memory_map();
    }

    #[cfg(test)]
    test_main();

    bit_os::hlt_loop()
}

global_asm!(include_str!("asm.s"));
extern "C" {
    pub fn jump(_rdi: u64);
}


fn kernel_start_message() {
    let msg =
"
/////////////////////////
/                       /
/   Welcome to bitOS!   /
/                       /
/////////////////////////
";
    set_color(ColorCode::new(Color::Cyan, Color::Black));
    println!("\n{}", msg);
    set_color(*VGA_COLOR);
}

fn load_feature(func: impl Fn(), text: &'static str) {
    print!("  {} ", text);
    let mut col_pos = vga_buffer::WRITER.lock().col_pos();
    while col_pos < 16 {
        print!(" ");
        col_pos += 1;
    }
    func();
    set_color(ColorCode::new(Color::Green, Color::Black));
    println!("[ok]");
    set_color(*VGA_COLOR);
}

use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};
fn _demonstrate_heap() {
    println!("Demonstrating heap");
    let heap_value = Box::new(42);
    println!("heap_value at {:p}", heap_value);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));

    println!("No crash!");
}

#[allow(dead_code)]
struct Link {
    prev: Option<Box<Link>>,
}

fn _cause_heap_overflow() {
    println!("This should cause a heap overflow");
    let genesis = Link{prev: None};
    let mut curr = Box::new(genesis);
    for _ in 0..1000000 {
        curr = Box::new(Link{prev: Some(curr)});
    }
    println!("Somehow this didn't cause a heap overflow");
}

fn _cause_page_fault() {
    let ptr = 0x20301b as *mut u32;
    unsafe {*ptr = 0; }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    set_color(ColorCode::new(Color::Red, Color::Black));
    println!("\n{}", info);
    set_color(*VGA_COLOR);
    bit_os::hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    bit_os::test_panic_handler(info)
}

