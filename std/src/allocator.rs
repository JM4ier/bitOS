use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use spin::Once;
use linked_list_allocator::*;
use dep::consts::*;

#[global_allocator]
static allocator: LazyAllocator = LazyAllocator;

static heap: Once<LockedHeap> = Once::new();

struct LazyAllocator;

unsafe fn init_heap() -> LockedHeap {
    LockedHeap::new(USER_HEAP_START as _, USER_HEAP_SIZE as _)
}

unsafe impl GlobalAlloc for LazyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        heap.call_once(|| init_heap()).alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        heap.call_once(|| init_heap()).dealloc(ptr, layout)
    }
}

