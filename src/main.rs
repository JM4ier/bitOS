#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rust_os::{print, println, memory};
use bootloader::{BootInfo, entry_point};
use x86_64::{VirtAddr, structures::paging::Page};

extern crate alloc;
use rust_os::allocator;

entry_point!(kernel_main);
pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Boot started");
    rust_os::init();
    println!("Boot stage 1 completed");

    // initializing kernel heap
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe{ memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed");
    println!("Boot stage 2 completed");
    println!("Boot complete\n");

    demonstrate_heap();
    //cause_heap_overflow();

    #[cfg(test)]
    test_main();

    rust_os::hlt_loop()
}

use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};
fn demonstrate_heap() {
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

struct Link {
    prev: Option<Box<Link>>,
}

fn cause_heap_overflow() {
    println!("This should cause a heap overflow");
    let genesis = Link{prev: None};
    let mut curr = Box::new(genesis);
    for _ in 0..1000000 {
        curr = Box::new(Link{prev: Some(curr)});
    }
    println!("Somehow this didn't cause a heap overflow");
}

fn cause_page_fault() {
    let ptr = 0x20301b as *mut u32;
    unsafe {*ptr = 0; }
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
