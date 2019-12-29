use crate::fs;
use super::*;
use core::default::Default;

#[derive(Copy, Clone)]
#[repr(C)]
#[repr(align(32))]
pub struct BlockGroupDescriptor {
    pub block_usage_table: RawBlockAddr,
    pub node_usage_table: RawBlockAddr,
    pub nodes: RawBlockAddr,
    pub unused_blocks: u16,
    pub unused_nodes: u16,
    pub dirs: u16,
}

impl Default for BlockGroupDescriptor {
    fn default () -> Self {
        Self {
            block_usage_table: RawBlockAddr::null(),
            node_usage_table: RawBlockAddr::null(),
            nodes: RawBlockAddr::null(),
            unused_blocks: 0,
            unused_nodes: 0,
            dirs: 0,
        }
    }
}

pub const BGD_PER_BLOCK: usize = BLOCK_SIZE / 32;
pub type BlockGroupDescriptorBlock = [BlockGroupDescriptor; BGD_PER_BLOCK];
pub type Block = [u8; BLOCK_SIZE];

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct RawBlockAddr {
    addr: u64,
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct BlockAddr {
    addr: u64,
}

impl RawBlockAddr {

    /// returns a 'null' address, which is `u64::max_value()`
    pub fn null() -> Self {
        Self {
            addr: u64::max_value(),
        }
    }

    /// checks if the address is 'null'
    pub fn is_null(self) -> bool {
        self.addr == RawBlockAddr::null().addr
    }

    /// increases the raw address by one, panics if the address is 'null'
    pub fn inc(self) -> Self {
        if self.is_null() {
            panic!("Trying to increase a null value");
        }
        Self {
            addr: self.addr + 1,
        }
    }

    /// returns the raw address
    pub fn as_u64(self) -> u64 {
        self.addr
    }
}

pub fn superblock_addr() -> RawBlockAddr {
    RawBlockAddr {
        addr: 1,
    }
}

/// translates block addresses to 'real' addresses by using the block group description table
pub fn translate_addr(device: &mut fs::BlockDevice<fs::bs::Size4KiB, BLOCK_SIZE>, addr: BlockAddr) -> Option<RawBlockAddr> {
    // TODO
    None
}

