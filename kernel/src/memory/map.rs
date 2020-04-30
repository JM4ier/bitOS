use spin::{Mutex, MutexGuard};
use x86_64::structures::paging::{*, mapper::*};
use x86_64::{PhysAddr, VirtAddr};

pub fn map(virt_addr: u64, flags: PageTableFlags) -> Result<u64, MapToError> {
    let table = unsafe { super::active_level_4_table() };
    let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virt_addr));
    let frame = super::allocator().allocate_frame().expect("out of memory");
    unsafe {
        super::mapper().map_to(page, frame, flags, &mut *super::allocator())?.flush();
    }
    Ok(frame.start_address().as_u64())
}

pub fn map_range(start: u64, end: u64, flags: PageTableFlags)  -> Result<(), MapToError> {
    for page in (start..=end).step_by(4096) {
        map(page, flags)?;
    }
    Ok(())
}

use crate::serial_println;
pub fn map_range_ignore_err(start: u64, end: u64, flags: PageTableFlags) {
    for page in (start..=end).step_by(4096) {
        let res = map(page, flags);
        match res {
            Ok(_) => (),
            Err(err) => serial_println!("Error {:?} while mapping {}", err, page),
        };
    }
}

