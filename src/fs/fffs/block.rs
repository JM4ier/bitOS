use super::*;
use core::default::Default;
use core::iter::Iterator;

#[derive(Copy, Clone)]
#[repr(C)]
#[repr(align(32))]
pub struct BlockGroupDescriptor {
    pub group_begin: RawAddr,
    pub unused_blocks: u16,
    pub unused_nodes: u16,
    pub dirs: u16,
}

impl Default for BlockGroupDescriptor {
    fn default () -> Self {
        Self {
            group_begin: RawAddr::null(),
            unused_blocks: 0,
            unused_nodes: 0,
            dirs: 0,
        }
    }
}

impl BlockGroupDescriptor {

    pub fn new (group_begin: RawAddr, unused_blocks: u16, unused_nodes: u16, dirs: u16) -> Self {
        Self {
            group_begin,
            unused_blocks,
            unused_nodes,
            dirs,
        }
    }

    pub fn block_usage_address(&self) -> RawAddr {
        self.group_begin
    }

    pub fn node_usage_address(&self) -> RawAddr {
        self.group_begin.offset(1)
    }

    pub fn node_blocks_begin(&self) -> RawAddr {
        self.group_begin.offset(2)
    }

    pub fn node_blocks_end(&self, supr: &SuperBlock) -> RawAddr {
        self.group_begin.offset(2 + supr.node_reserved_blocks_per_group() as i64)
    }

    pub fn usable_blocks_begin(&self, supr: &SuperBlock) -> RawAddr {
        self.node_blocks_end(supr)
    }
}

pub const BGD_PER_BLOCK: usize = BLOCK_SIZE / 32;
pub type BlockGroupDescriptorBlock = [BlockGroupDescriptor; BGD_PER_BLOCK];
pub type Block = [u8; BLOCK_SIZE];

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct RawAddr {
    addr: u64,
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct BlockAddr {
    addr: u64,
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct NodeAddr {
    addr: u64,
}

impl RawAddr {

    /// returns a 'null' address, which is `u64::max_value()`
    pub fn null() -> Self {
        Self {
            addr: u64::max_value(),
        }
    }

    pub fn new(addr: u64) -> Self {
        Self {
            addr,
        }
    }

    /// checks if the address is 'null'
    pub fn is_null(self) -> bool {
        self.addr == Self::null().addr
    }

    /// increases the raw address by `offset`, panics if the address is 'null'
    pub fn offset(self, offset: i64) -> Self {
        if self.is_null() {
            panic!("Trying to increase a null value");
        }
        Self {
            addr: (self.addr as i64 + offset) as u64,
        }
    }

    /// returns the raw address
    pub fn as_u64(self) -> u64 {
        self.addr
    }

    pub fn until (self, end: RawAddr) -> RawAddrIter {
        RawAddrIter {
            start: self,
            end,
        }
    }
}

impl BlockAddr {
    pub fn null() -> Self {
        Self {
            addr: u64::max_value(),
        }
    }
    pub fn is_null(&self) -> bool {
        self.addr == Self::null().addr
    }
    pub fn inner_u64(&self) -> u64 {
        self.addr
    }
}

impl NodeAddr {
    pub fn null() -> Self {
        Self {
            addr: u64::max_value(),
        }
    }
    pub fn is_null(&self) -> bool {
        self.addr == Self::null().addr
    }
    pub fn inner_u64(&self) -> u64 {
        self.addr
    }
}

pub fn superblock_addr() -> RawAddr {
    RawAddr {
        addr: 1,
    }
}

pub struct RawAddrIter {
    start: RawAddr,
    end: RawAddr,
}

impl Iterator for RawAddrIter {
    type Item = RawAddr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start.as_u64() == self.end.as_u64() {
            None
        } else {
            let item = self.start;
            self.start = self.start.offset(1);
            Some(item)
        }
    }
}

