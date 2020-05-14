use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags,
    },
    VirtAddr,
};

pub const HEAP_START: usize = 0x4444_4444_0000;
pub const HEAP_SIZE: usize = 128 * 1024 * 1024; // 128M

use linked_list_allocator::LockedHeap;

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

pub fn init_heap() -> Result<(), MapToError> {
    let mut mapper = unsafe { super::mapper() };
    let mut frame_allocator = super::allocator();

    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    unsafe {
        for page in page_range {
            let frame = frame_allocator.allocate_frame()
                .ok_or(MapToError::FrameAllocationFailed)?;
            mapper.map_to(page, frame, flags, &mut *frame_allocator)?.flush();
        }
    }


    unsafe {
        HEAP.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

