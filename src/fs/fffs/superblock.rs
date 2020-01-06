use super::*;
use core::mem;

/// randomly generated number to identify the superblock
pub const MAGIC: u64 = 5172077894053490781;

/// max length of volume name
pub const VOLUME_NAME_LEN: usize = 32;

/// Superblock of file system.
/// It stores general informations about the file system
/// such as block size, size of the volume, etc
#[repr(C, align(4096))]
pub struct SuperBlock {
    /// signature to identify superblock
    pub magic: u64,

    /// total number of nodes
    pub nodes: u64,

    /// total number of blocks
    pub blocks: u64,

    /// number of blocks reserved for system
    pub reserved: u64,

    /// number of free nodes
    pub free_nodes: u64,

    /// number of free blocks
    pub free_blocks: u64,

    /// block index of block containing superblock
    pub super_block_index: RawAddr,

    /// block size (this field is at the moment always 4096, but maybe it will be variable one day
    pub block_size: u64,

    /// Size of a node in bytes
    pub node_size: u64,

    /// number of blocks in block group
    pub block_group_size: u64,

    /// number of nodes in block group
    pub block_group_node_count: u64,

    /// last mount time
    pub last_mount: Time,

    /// last write time
    pub last_write: Time,

    /// amount of times the fs has been mounted
    pub mount_count: u64,

    /// number of times the fs can be mounted before being checked
    pub mount_check: u64,

    /// volume name
    pub name: [u8; VOLUME_NAME_LEN],
}

impl SuperBlock {

    /// creates dummy `SuperBlock` with size `0`.
    pub fn empty () -> Self {
        Self::new(0, [b' '; VOLUME_NAME_LEN])
    }

    /// creates new superblock
    pub fn new (blocks: u64, name: [u8; VOLUME_NAME_LEN]) -> Self {
        let block_group_count = blocks / BLOCK_GROUP_SIZE;
        let bgdt_reserved = block_group_count * mem::size_of::<block::BlockGroupDescriptor>() as u64
            / BLOCK_SIZE as u64;
        let nodes = block_group_count * NODES_PER_GROUP;
        let reserved = 16 + bgdt_reserved;
        use crate::println;
        println!("b: {}, r:{}", blocks, reserved);
        Self {
            magic: MAGIC,
            nodes,
            blocks,
            reserved,
            free_nodes: 0,
            free_blocks: blocks - reserved,
            super_block_index: superblock_addr(),
            block_size: BLOCK_SIZE as u64,
            node_size: mem::size_of::<Node>() as u64,
            block_group_size: BLOCK_GROUP_SIZE,
            block_group_node_count: NODES_PER_GROUP,
            last_mount: 0, // TODO current time
            last_write: 0, // TODO current time
            mount_count: 0,
            mount_check: 32,
            name,
        }
    }

    /// Returns `true` when the `magic` value is correct.
    /// This is useful for finding the super block on an unknown device
    pub fn is_valid(&self) -> bool {
        self.magic == MAGIC
    }


    /// Update superblock stats of mounting
    pub fn mark_mounted(&mut self) {
        self.mount_count += 1;
        // TODO update self.last_mount
    }

    /// reserved blocks for node table in a single block group
    pub fn node_reserved_blocks_per_group (&self) -> u64 {
        self.block_group_node_count * self.node_size / self.block_size
    }

    /// how many usable blocks there are per group
    pub fn usable_blocks_per_group (&self) -> u16 {
        // total blocks per group - reserved for nodes - usage tables of nodes and usable blocks
        (self.block_group_size - self.node_reserved_blocks_per_group() - 2) as u16
    }

    /// returns the number of block groups on the volume
    pub fn block_group_count (&self) -> u64 {
        self.blocks / self.block_group_size
    }
}

