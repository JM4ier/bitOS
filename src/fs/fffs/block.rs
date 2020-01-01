use super::*;
use core::default::Default;
use core::iter::Iterator;

#[derive(Copy, Clone)]
#[repr(C, align(32))]
pub struct BlockGroupDescriptor {
    /// first block that belongs to the group
    pub group_begin: RawAddr,

    /// number of unused blocks in the group
    pub unused_blocks: u16,

    /// number of unused nodes in the group
    pub unused_nodes: u16,

    /// number of directories in the group
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

    /// returns a new `BlockGroupDescriptor` with the given arguments
    pub fn new (group_begin: RawAddr, unused_blocks: u16, unused_nodes: u16, dirs: u16) -> Self {
        Self {
            group_begin,
            unused_blocks,
            unused_nodes,
            dirs,
        }
    }

    /// block address of the block usage table
    pub fn block_usage_address(&self) -> RawAddr {
        self.group_begin
    }

    /// block address of the node usage table
    pub fn node_usage_address(&self) -> RawAddr {
        self.group_begin.offset(1)
    }

    /// address of the first block containing node table data
    pub fn node_blocks_begin(&self) -> RawAddr {
        self.group_begin.offset(2)
    }

    /// address of the first block after the node table
    pub fn node_blocks_end(&self, supr: &SuperBlock) -> RawAddr {
        self.group_begin.offset(2 + supr.node_reserved_blocks_per_group() as i64)
    }

    /// address of the first block in the group that can store files/directories etc
    pub fn usable_blocks_begin(&self, supr: &SuperBlock) -> RawAddr {
        self.node_blocks_end(supr)
    }
}

/// block group descriptor struct memory size
pub const BGD_SIZE: usize = 32;

/// block group descriptors per block
pub const BGD_PER_BLOCK: usize = BLOCK_SIZE / BGD_SIZE;

pub type BlockGroupDescriptorBlock = [BlockGroupDescriptor; BGD_PER_BLOCK];
pub type Block = [u8; BLOCK_SIZE];

/// raw address that describes the location of a block
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct RawAddr {
    addr: u64,
}

/// Address of a block in the file system.
/// It combines the number of the block group
/// and the location of the block inside the group.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct BlockAddr {
    addr: u64,
}

/// Address of a node in the file system.
/// It combines the number of the block group that contains the node
/// with the location of the node inside the block group.
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

    /// returns a new address with the given location
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

    /// returns an `RawAddrIter` that iterates over every `RawAddr` between `self` and `end`.
    pub fn until (self, end: RawAddr) -> RawAddrIter {
        RawAddrIter {
            start: self,
            end,
        }
    }
}

impl BlockAddr {
    /// returns a 'null' value address
    pub fn null() -> Self {
        Self {
            addr: u64::max_value(),
        }
    }

    /// returns true if the address is 'null'
    pub fn is_null(&self) -> bool {
        self.addr == Self::null().addr
    }

    /// Returns the inner, stored address. Don't use this to index a block device,
    /// the address needs to be translated first using a block group descriptor table.
    pub fn inner_u64(&self) -> u64 {
        self.addr
    }
}

impl NodeAddr {
    /// returns a 'null' value address
    pub fn null() -> Self {
        Self {
            addr: u64::max_value(),
        }
    }

    /// returns true if the address is 'null'
    pub fn is_null(&self) -> bool {
        self.addr == Self::null().addr
    }

    /// Returns the inner, stored address. Don't use this to index a block device,
    /// the address needs to be translated first using a block group descriptor table.
    pub fn inner_u64(&self) -> u64 {
        self.addr
    }
}

/// returns the location of the file systems superblock
pub fn superblock_addr() -> RawAddr {
    RawAddr {
        addr: 1,
    }
}

/// returns the first block that contains the file systems group descriptor table
pub fn gdt_addr() -> RawAddr {
    superblock_addr().offset(1)
}

/// `Iterator` for `RawAddr`
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

