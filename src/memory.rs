use x86_64::{
    structures::paging::{
        Page,
        PhysFrame,
        Mapper,
        Size4KiB,
        FrameAllocator,
        PageTable,
        OffsetPageTable,
    },
    VirtAddr,
    PhysAddr,
};

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

pub fn create_example_mapping (
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe {
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("mapping failed").flush();
}

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    region: usize,
    frame: u64,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            region: 0,
            frame: 0,
        }
    }
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    fn region_oob(&self) -> bool {
        self.region >= self.memory_map.len()
    }
    fn region_usable(&self) -> bool {
        !self.region_oob() && self.memory_map[self.region].region_type == MemoryRegionType::Usable
    }
    fn increase_region(&mut self) {
        self.region += 1;
        self.frame = 0;
        while !self.region_oob() && !self.region_usable() {
            self.region += 1;
        }
    }
    fn increase_frame(&mut self) {
        self.frame += 1;
        if !self.region_usable() {
            self.increase_region();
        }
        let region = self.memory_map[self.region];
        let range = region.range;
        if range.start_addr() + 4096 * self.frame >= range.end_addr() {
            self.increase_region();
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        //self.frame += 1;
        //self.usable_frames().nth(self.frame as usize)
        self.increase_frame();
        if self.region_oob() {
            None
        } else {
            Some(PhysFrame::containing_address(PhysAddr::new(
                self.memory_map[self.region].range.start_addr() + 4096 * self.frame
            )))
        }
    }
}

