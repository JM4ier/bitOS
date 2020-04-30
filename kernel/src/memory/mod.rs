use x86_64::{
    structures::paging::{
        *,
        page_table::*,
    },
    VirtAddr,
    PhysAddr,
};
use spin::{Mutex, MutexGuard, Once};
use bootloader::BootInfo;

pub mod heap;
mod allocator;
mod map;

use allocator::*;
pub use map::*;

static mut BOOT_INFO: Once<&'static BootInfo> = Once::new();
static mut ALLOCATOR: Once<Mutex<BootInfoFrameAllocator>> = Once::new();

pub fn init_boot_info(boot_info: &'static BootInfo) {
    unsafe {
        BOOT_INFO.call_once(|| boot_info);
    }
}

pub fn boot_info() -> &'static BootInfo {
    unsafe {
        &BOOT_INFO.r#try().expect("uninitialized boot info")
    }
}

pub fn init_allocator() {
    allocator();
}

pub fn allocator() -> MutexGuard<'static, BootInfoFrameAllocator> {
    unsafe {
        ALLOCATOR.call_once(||
            Mutex::new(BootInfoFrameAllocator::init(&boot_info().memory_map))
        ).lock()
    }
}

pub unsafe fn mapper() -> OffsetPageTable<'static> {
    let physical_memory_offset = VirtAddr::new(boot_info().physical_memory_offset);
    let level_4_table = active_level_4_table();
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table() -> &'static mut PageTable {
    let physical_memory_offset = VirtAddr::new(boot_info().physical_memory_offset);

    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

unsafe fn new_table(id: u64) -> u64 {
    let page_addr = 0; // FIXME virt_addr dependent  on id
    let frame = map(page_addr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE).unwrap();

    let new_p4 = &mut *(page_addr as *mut PageTable);
    let current_p4 = active_level_4_table();

    // initialize empty page table
    for i in 0..512 {
        new_p4[i] = PageTableEntry::new();
    }

    // copy higher half of memory mapping (kernel space)
    for i in 256..512 {
        new_p4[i] = current_p4[i].clone();
    }

    frame
}

fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    let mem_offset = VirtAddr::new(boot_info().physical_memory_offset);
    mem_offset + phys.as_u64()
}

use crate::serial_println;
pub fn print_memory_map() {
    let memory_map = allocator().memory_map;
    serial_println!("{:?}", memory_map);
}

pub fn print_virt_memory_map() {
    let p4 = unsafe { active_level_4_table() };
    let mut last_region = None;
    serial_println!("   ---  Virtual Memory Used  ---   ");
    find_virt_mem_regions(4, VirtAddr::new(0), &*p4, &mut last_region);
    if let Some(region) = last_region {
        serial_println!("{:?}", region);
    }
}

fn find_virt_mem_regions(lvl: usize, offset: VirtAddr, table: &PageTable, last_region: &mut Option<(VirtAddr, VirtAddr)>) {
    let mut span = 4096;
    for _ in 1..lvl {
        span *= 512;
    }

    for (i, entry) in table.iter().enumerate() {
        if entry.is_unused() {
            continue;
        }
        let offset = offset + i * span;
        if entry.flags().contains(PageTableFlags::HUGE_PAGE) || lvl == 1 {
            // maps memory
            let region = (offset, offset+span);
            if let Some(prev) = last_region {
                if prev.1 == region.0 {
                    *last_region = Some((prev.0, region.1));
                } else {
                    serial_println!("{:?}", prev);
                    *last_region = Some(region);
                }
            } else {
                *last_region = Some(region);
            }
        } else {
            let table: *const PageTable = phys_to_virt(entry.addr()).as_ptr();
            find_virt_mem_regions(lvl-1, offset, unsafe{&*table}, last_region);
        }
    }
}

