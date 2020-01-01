//! first fucking file system

use core::default::Default;
use crate::fs::{self, Path, AsU8Slice, FsResult, FsError, BlockDevice, SerdeBlockDevice};
use alloc::{vec, vec::Vec, boxed::Box};


pub mod perm;
use self::perm::*;

pub mod superblock;
use self::superblock::*;

pub mod block;
use self::block::*;

pub mod node;
use self::node::*;

pub mod pointer;
use self::pointer::*;

pub const BLOCK_SIZE: usize = 4096;
pub const BLOCK_GROUP_SIZE: u64 = 8192;
pub const NODES_PER_GROUP: u64 = 1536;

pub type Time = i64;

pub struct FileSystem {
    device: Box<dyn BlockDevice>,
    superblock: SuperBlock,
}

impl FileSystem {
    /// creates a new fs on the given `BlockDevice`.
    pub fn format(mut device: impl BlockDevice + 'static, name: &[u8]) -> FsResult<FileSystem> {
        let mut part_name = [0; VOLUME_NAME_LEN];
        for (i, b) in name.iter().enumerate() {
            if i >= VOLUME_NAME_LEN {
                break;
            }
            part_name[i] = *b;
        }
        let mut superblock = SuperBlock::new(device.blocks(), part_name);
        device.write(superblock_addr().as_u64(), &mut superblock)?;
        let mut file_system = Self {
            device: Box::new(device),
            superblock,
        };
        file_system.init_block_groups()?;
        Ok(file_system)
    }

    /// number of blocks the block group descriptor table needs
    fn bgdt_block_count(&self) -> u64 {
        self.superblock.block_group_count() / BGD_PER_BLOCK as u64
    }

    /// Initializes the block group descriptor table and all the block groups
    fn init_block_groups(&mut self) -> FsResult<()> {
        // create the entries for
        for i in 0..self.bgdt_block_count() {
            let mut bgdt = [BlockGroupDescriptor::default(); BGD_PER_BLOCK];
            for k in 0..BGD_PER_BLOCK {
                let n = i * BGD_PER_BLOCK as u64 + k as u64;
                bgdt[k] = self.create_bg_desc(n)?;
            }
            self.device.write_block(i + 1, bgdt.as_u8_slice())?;
        }
        Ok(())
    }

    /// Initializes the block group with the specified `index` and returns the associated
    /// `BlockGroupDescriptor`.
    fn create_bg_desc (&mut self, index: u64) -> FsResult<BlockGroupDescriptor> {
        let supr = &mut self.superblock;
        let reserved_offset = supr.reserved;
        let group_size = supr.block_group_size;
        let group_offset = reserved_offset + index * group_size;

        // empty block used to overwrite blocks on the blockdevice
        let empty_block = vec![0u8; supr.block_size as usize].into_boxed_slice();

        let descriptor = BlockGroupDescriptor::new(
            RawAddr::new(group_offset),
            supr.usable_blocks_per_group(),
            supr.block_group_node_count as u16,
            0,
            );

        // override node usage table
        self.device.write_block(descriptor.node_usage_address().as_u64(), &empty_block)?;

        let mut reserved_block_bitmap = empty_block;
        for (i, _node) in descriptor.node_blocks_begin().until(descriptor.node_blocks_end(&supr)).enumerate() {
            set_bit(&mut reserved_block_bitmap, 2 + i as usize, true);
            // not needed to clear the node
        }

        // overwrite block usage buffer to reserve blocks for the node table
        self.device.write_block(descriptor.node_usage_address().as_u64(), &reserved_block_bitmap)?;

        Ok(descriptor)
    }

    fn group_descriptor(&mut self, group: u64) -> FsResult<BlockGroupDescriptor> {
        if group > self.superblock.block_group_count() {
            Err(FsError::InvalidAddress)
        } else {
            // descriptor table starts at block 1 (superblock is block 0)
            let descriptor_block = group / BGD_PER_BLOCK as u64 + 1;
            let descriptor_index = group % BGD_PER_BLOCK as u64;

            // read descriptor table
            let mut desc_table = [BlockGroupDescriptor::default(); BGD_PER_BLOCK];
            self.device.read_block(descriptor_block, desc_table.as_u8_slice_mut())?;
            let descriptor = desc_table[descriptor_index as usize];
            Ok(descriptor)
        }
    }

    fn translate_block_addr(&mut self, addr: BlockAddr) -> FsResult<RawAddr> {
        let addr = addr.inner_u64();
        let bpg = self.superblock.usable_blocks_per_group() as u64;

        let group = addr / bpg;
        let index = addr % bpg;

        let descriptor = self.group_descriptor(group)?;

        // TODO check if block is used

        // return block
        Ok(descriptor.usable_blocks_begin(&self.superblock).offset(index as i64))
    }

    fn translate_node_addr(&mut self, addr: NodeAddr) -> FsResult<Node> {
        let addr = addr.inner_u64();

        let group = addr / NODES_PER_GROUP;
        let index = addr % NODES_PER_GROUP;

        let descriptor = self.group_descriptor(group)?;

        // TODO check if node is used

        let block = index / self.superblock.node_reserved_blocks_per_group(); // block of the groups node table
        let index = index % self.superblock.node_reserved_blocks_per_group(); // index inside the block

        // read the node table
        let mut node_table = node_table();
        self.device.read_block(descriptor.node_blocks_begin().offset(block as i64).as_u64(), &mut node_table.as_u8_slice_mut())?;

        Ok(node_table[index as usize])
    }

    fn read_node_content(&mut self, node: Node) -> FsResult<Vec<Block>> {
        let mut blocks = Vec::new();

        let depth = node.indirection_level as usize + 1;
        let mut stack = Vec::with_capacity(depth);
        let mut pstack: Vec<Vec<BlockAddr>> = Vec::with_capacity(depth);

        stack.push(0);
        pstack.push(node.pointers.to_vec());

        while !stack.is_empty() {
            let addr = pstack.last().unwrap()[*stack.last().unwrap()];

            if !addr.is_null() {
                let addr = self.translate_block_addr(addr)?;

                if stack.len() < depth {
                    // read pointer block
                    let mut pointer_data = PointerData::empty();
                    self.device.read_block(addr.as_u64(), &mut pointer_data.as_u8_slice_mut())?;
                    pstack.push(pointer_data.pointers.to_vec());
                    stack.push(0);
                    continue;
                } else {
                    // read content block
                    let mut block = [0u8; BLOCK_SIZE];
                    self.device.read_block(addr.as_u64(), &mut block)?;
                    blocks.push(block);
                }

                // increase index after reading a data block or when a pointer is null
                while !stack.is_empty() {
                    let index = stack.pop().unwrap() + 1;
                    if index >= pstack.last().unwrap().len() {
                        pstack.pop();
                    } else {
                        stack.push(index);
                        break;
                    }
                }
            }
        }

        Ok(blocks)
    }

    fn allocate_block() -> BlockAddr {
        panic!("Unimplemented");
    }

    fn deallocate_block(block: BlockAddr) {
        panic!("Unimplemented");
    }
}



// TODO
impl fs::FileSystem for FileSystem {
    fn allowed_chars() -> &'static [u8] {
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-."
    }

    fn separator() -> u8 {
        b'/'
    }

    /// takes the given blockdevice and tries to read it as this fs
    fn mount<D: BlockDevice + 'static>(device: D) -> FsResult<FileSystem> {
        let mut device = device;
        let mut superblock = SuperBlock::empty();
        device.read(superblock_addr().as_u64(), &mut superblock)?;
        if superblock.is_valid() {
            superblock.mark_mounted();
            device.write(superblock_addr().as_u64(), &mut superblock)?;
            let file_system = Self {
                device: Box::new(device),
                superblock,
            };
            // TODO load bgdt and initialize stuff
            Ok(file_system)
        } else {
            Err(FsError::InvalidSuperBlock)
        }
    }

    fn open(path: Path) -> Result<i64, FsError> {
        panic!("Not implemented");
    }

    fn delete(path: Path) -> FsResult<()> {
        panic!("Not implemented");
    }

    fn clear(path: Path) -> FsResult<()> {
        panic!("Not implemented");
    }

    fn create_file(path: Path) -> FsResult<()> {
        panic!("Not implemented");
    }

    fn create_directory(path: Path) -> FsResult<()> {
        panic!("Not implemented");
    }

    fn exists_file(path: Path) -> FsResult<bool> {
        panic!("Not implemented");
    }

    fn exists_directory(path: Path) -> FsResult<bool> {
        panic!("Not implemented");
    }
}

fn set_bit(bitmap: &mut [u8], index: usize, value: bool) {
    let byte = index / 8;
    let bit = index % 8;
    let bitmask = 1 << bit;
    if value {
        bitmap[byte] |= bitmask;
    } else {
        bitmap[byte] &= !bitmask;
    }
}

fn get_bit(bitmap: &mut [u8], index: usize) -> bool {
    let byte = index / 8;
    let bit = index % 8;
    let bitmask = 1 << bit;
    bitmap[byte] & bitmask > 0
}

#[test_case]
fn test_fffs_struct_sizes () {
    use core::mem::size_of;
    use crate::{serial_print, serial_println};
    serial_print!("test_fffs_struct_sizes... ");
    assert_eq!(size_of::<SuperBlock>(), 4096);
    assert_eq!(size_of::<BlockGroupDescriptorBlock>(), 4096);
    assert_eq!(size_of::<PointerData>(), 4096);
    assert_eq!(size_of::<Block>(), 4096);
    assert_eq!(size_of::<Node>(), 128);
    assert_eq!(size_of::<PointerData>(), 4096);
    assert_eq!(size_of::<NodeType>(), 1);
    assert_eq!(size_of::<BlockGroupDescriptor>(), 32);
    assert_eq!(size_of::<RawAddr>(), 8);
    assert_eq!(size_of::<BlockAddr>(), 8);
    assert_eq!(size_of::<Permission>(), 2);
    serial_println!("[ok]");
}

