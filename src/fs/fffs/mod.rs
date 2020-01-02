//! first fucking file system

use core::default::Default;
use crate::fs::{self, Path, AsU8Slice, FsResult, FsError, BlockDevice, SerdeBlockDevice};
use alloc::{vec, vec::Vec, boxed::Box};


// modules
pub mod perm;
use perm::*;

pub mod superblock;
use superblock::*;

pub mod block;
use block::*;

pub mod node;
use node::*;

pub mod pointer;
use pointer::*;

pub mod bits;
use bits::*;



/// Block size of the file system. At the moment there is only one supported blocksize
pub const BLOCK_SIZE: usize = 4096;

/// Number of blocks belonging to a block group.
/// This includes blocks used by the block group for
/// management purposes, like a block usage table.
pub const BLOCK_GROUP_SIZE: u64 = 8192;

/// Number of nodes in a group.
pub const NODES_PER_GROUP: u64 = 1536;

/// Using a 64-bit integer to store time, to evade the year 38 problem.
/// (Although it probably isn't relevant as there is a close to zero chance
/// that this kernel is ever gonna be used)
pub type Time = i64;

/// The file system struct that acts as an interface to the rest of the kernel
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
        device.write(superblock_addr(), &mut superblock)?;
        let mut file_system = Self {
            device: Box::new(device),
            superblock,
        };
        file_system.init_block_groups()?;

        // create root directory
        let root = {
            let mut node = Node::null();
            node.node_type = NodeType::Directory;
            node
        };

        let addr = file_system.create_node_in_group(0)?;
        file_system.write_node(addr, root)?;
        file_system.write_node_content(addr, root, DirectoryData::empty().to_blocks())?;

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
            let addr = gdt_addr().offset(i as _);
            self.device.write_block(addr.as_u64(), bgdt.as_u8_slice())?;
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

    /// returns the group descriptor table, the block of the descriptor and the index in the block to the group passed as argument
    fn group_descriptor_table(&mut self, group: u64) -> FsResult<(BlockGroupDescriptorBlock, u64, usize)> {
        if group > self.superblock.block_group_count() {
            Err(FsError::InvalidAddress)
        } else {
            let descriptor_block = group / BGD_PER_BLOCK as u64;
            let descriptor_index = group % BGD_PER_BLOCK as u64;

            // offset the descriptor block with the begin of the group descriptor table
            let descriptor_block = gdt_addr().offset(descriptor_block as _).as_u64();

            // read descriptor table
            let mut desc_table = [BlockGroupDescriptor::default(); BGD_PER_BLOCK];
            self.device.read(descriptor_block, &mut desc_table)?;

            Ok((desc_table, descriptor_block, descriptor_index as usize))
        }
    }

    fn group_descriptor(&mut self, group: u64) -> FsResult<BlockGroupDescriptor> {
        let (table, _, index) = self.group_descriptor_table(group)?;
        let descriptor = table[index];
        Ok(descriptor)
    }

    /// returns group index and block index in group
    fn block_addr_to_group_index(&mut self, addr: BlockAddr) -> (u64, u64) {
        let addr = addr.inner_u64();
        let bpg = self.superblock.usable_blocks_per_group() as u64;

        let group = addr / bpg;
        let index = addr % bpg;

        (group as u64, index as u64)
    }

    /// translates a block address to a raw address using the group descriptor table
    fn translate_block_addr(&mut self, addr: BlockAddr) -> FsResult<RawAddr> {
        let (group, index) = self.block_addr_to_group_index(addr);
        let descriptor = self.group_descriptor(group)?;

        // load block usage table
        let mut usage_table = [0u8; BLOCK_SIZE];
        self.device.read(descriptor.block_usage_address(), &mut usage_table)?;

        // if the block is unused, return error
        if get_bit(&usage_table, index as usize) {
            Ok(descriptor.usable_blocks_begin(&self.superblock).offset(index as i64))
        } else {
            Err(FsError::InvalidIndex)
        }
    }

    /// returns (table, raw block addr as usize, node addr in table
    fn read_node_table(&mut self, addr: NodeAddr) -> FsResult<(NodeTable, u64, usize)> {
        let addr = addr.inner_u64();

        let group = addr / NODES_PER_GROUP;
        let index = addr % NODES_PER_GROUP;

        let descriptor = self.group_descriptor(group)?;

        // TODO check if node is used

        let block = index / self.superblock.node_reserved_blocks_per_group(); // block of the groups node table
        let index = index % self.superblock.node_reserved_blocks_per_group(); // index inside the block

        let block_index = descriptor.node_blocks_begin().offset(block as i64).as_u64();

        // read the node table
        let mut node_table = node_table();
        self.device.read(block_index, &mut node_table)?;

        Ok((node_table, block_index, index as usize))
    }

    /// translates a node address to a raw address using the group descriptor table
    fn read_node(&mut self, addr: NodeAddr) -> FsResult<Node> {
        let (table, _,  index) = self.read_node_table(addr)?;
        Ok(table[index])
    }

    fn write_node(&mut self, addr: NodeAddr, node: Node) -> FsResult<()> {
        let (mut table, block_addr, index) = self.read_node_table(addr)?;
        table[index] = node;
        self.device.write(block_addr, &table)
    }

    /// reads the contents of a node and returns it as a `Vec` of `Block`s.
    /// In the case of a file, this returns the file split into `BLOCK_SIZE`
    /// big `u8` arrays.
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
                    self.device.read(addr, &mut pointer_data)?;
                    pstack.push(pointer_data.pointers.to_vec());
                    stack.push(0);
                    continue;
                } else {
                    // read content block
                    let mut block = [0u8; BLOCK_SIZE];
                    self.device.read(addr, &mut block)?;
                    blocks.push(block);
                }

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

        Ok(blocks)
    }

    /// writes the nodes contents blocks to the device
    fn write_node_content(&mut self, addr: NodeAddr, node: Node, content: Vec<Block>) -> FsResult<()> {
        let mut node = node;
        let indirection_level = {
            let mut i = 0;
            let mut blocks = POINTERS_PER_NODE;
            loop {
                if content.len() <= blocks {
                    break i;
                }
                blocks *= POINTERS_PER_BLOCK;
                i += 1;
            }
        };

        node.indirection_level = indirection_level;
        let depth = indirection_level as usize;
        let mut stack = Vec::with_capacity(depth);
        let mut pstack = Vec::with_capacity(depth);

        stack.push(0);
        pstack.push(node.pointers);

        while !stack.is_empty() {

            // TODO write data


            // increase index
            while !stack.is_empty() {
                let index = stack.pop().unwrap() + 1;
                if index >= pstack.last().unwrap().len() {
                    // table full
                    if pstack.len() == 1 {
                        // node pointers
                        // writing node at end anyways
                        pstack.pop();
                    } else {
                        // block pointers
                    }
                }
            }
        }

        // TODO write data

        panic!("Unimplemented");
    }

    /// returns the `NodeAddr` from the given path and returns errors if the node doesn't exist
    fn node_addr_from_path (&mut self, path: Path) -> FsResult<NodeAddr> {
        let mut addr = NodeAddr::root();
        for (i, p) in path.path.iter().enumerate() {
            let node = self.read_node(addr)?;
            match node.node_type {
                NodeType::File => {
                    return Err(FsError::FileNotFound);
                },
                NodeType::Directory => {
                    let data = self.read_node_content(node)?;
                    let data = DirectoryData::from_blocks(&node, &data);
                    let mut new_addr = NodeAddr::null();
                    for entry in data.entries.iter() {
                        if p == &entry.name {
                            new_addr = entry.addr;
                            break;
                        }
                    }
                    if new_addr.is_null() {
                        return Err(FsError::FileNotFound);
                    } else {
                        addr = new_addr;
                    }
                },
                _ => panic!("Not implemented"),
            }
        }
        Ok(addr)
    }

    fn node_from_path (&mut self, path: Path) -> FsResult<Node> {
        let addr = self.node_addr_from_path(path)?;
        let node = self.read_node(addr)?;
        Ok(node)
    }

    /// creates a node in the specified group or returns an error if there is no space or an other error occurs
    fn create_node_in_group(&mut self, group: u64) -> FsResult<NodeAddr> {
        let (mut desc_table, block_index, entry) = self.group_descriptor_table(group)?;
        let mut descriptor = desc_table[entry];

        if descriptor.unused_nodes > 0 {
            // read node usage table
            let mut node_usage = [0u8; BLOCK_SIZE];
            self.device.read(descriptor.node_usage_address(), &mut node_usage)?;

            for n in 0..NODES_PER_GROUP {
                if !get_bit(&node_usage, n as usize) {
                    // allocate node in block

                    // mark node as used in table
                    set_bit(&mut node_usage, n as usize, true);
                    self.device.write(descriptor.node_usage_address(), &node_usage)?;

                    // decrease unused nodes counter and write to block device
                    descriptor.unused_nodes -= 1;
                    desc_table[entry] = descriptor;
                    self.device.write(block_index, &desc_table)?;

                    return Ok(NodeAddr::new(group * NODES_PER_GROUP + n));
                }
            }
        }
        Err(FsError::InternalError)
    }

    /// creates a new node based on the parent node: It tries to put the new node in the same block group
    /// as te parent node, if this is not possible, it puts it in any other block group.
    fn create_node(&mut self, parent: NodeAddr) -> FsResult<NodeAddr> {
        let group = parent.inner_u64() / NODES_PER_GROUP;
        if let Ok(addr) = self.create_node_in_group(group) {
            return Ok(addr);
        }
        // find a block that still has free nodes
        // and return a new node from that block
        for group in 0..self.superblock.block_group_count() {
            if let Ok(addr) = self.create_node_in_group(group) {
                return Ok(addr);
            }
        }
        Err(FsError::NotEnoughSpace)
    }

    /// Tries to allocate a block in the specified group.
    /// Should there be a lack of space in the specified group,
    /// any group will be chosen.
    fn allocate_block(&mut self, preferred_group: u64) -> FsResult<BlockAddr> {
        if let Ok(addr) = self.allocate_block_in_group(preferred_group) {
            return Ok(addr);
        }

        let groups = self.superblock.block_group_count();
        for group in 0..groups {
            if let Ok(addr) = self.allocate_block_in_group(group) {
                return Ok(addr);
            }
        }
        Err(FsError::NotEnoughSpace)
    }

    /// tries to allocate a block in the specified group
    /// if there is no space in that group, an error is returned
    fn allocate_block_in_group(&mut self, group: u64) -> FsResult<BlockAddr> {
        let (mut desc_table, table_block, desc_index) = self.group_descriptor_table(group)?;
        let mut descriptor = desc_table[desc_index];

        // blocks per group
        let bpg = self.superblock.usable_blocks_per_group() as u64;

        if descriptor.unused_blocks > 0 {
            // read usage table
            let mut usage_table = [0u8; BLOCK_SIZE];
            self.device.read(descriptor.block_usage_address(), &mut usage_table)?;

            // find free block
            for i in 0..bpg {
                if !get_bit(&usage_table, i as usize) {
                    // edit usage table
                    set_bit(&mut usage_table, i as usize, true);
                    self.device.write(descriptor.block_usage_address(), &usage_table)?;

                    // edit descriptor
                    descriptor.unused_blocks -= 1;
                    desc_table[desc_index] = descriptor;
                    self.device.write(table_block, &desc_table)?;

                    return Ok(BlockAddr::new(group * bpg + i));
                }
            }
        }
        Err(FsError::InternalError)
    }

    fn clear_block(&mut self, addr: RawAddr) -> FsResult<()> {
        let empty = [0u8; BLOCK_SIZE];
        self.device.write(addr, &empty)?;
        Ok(())
    }

    fn deallocate_block(&mut self, block: BlockAddr) -> FsResult<()> {
        let (group, index) = self.block_addr_to_group_index(block);
        let index = index as usize;
        let mut descriptor = self.group_descriptor(group)?;

        // clear the block
        let addr = descriptor.usable_blocks_begin(&self.superblock).offset(index as _);
        self.clear_block(addr)?;

        // read usage table
        let mut usage_table = [0u8; BLOCK_SIZE];
        self.device.read(descriptor.block_usage_address(), &mut usage_table)?;

        if get_bit(&usage_table, index) {
            // mark block as unused and write usage table and group descriptor
            set_bit(&mut usage_table, index, false);
            descriptor.unused_blocks += 1;
            self.device.write(descriptor.block_usage_address(), &usage_table)?;
            // TODO write descriptor
            Ok(())
        } else {
            Err(FsError::InvalidIndex)
        }
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

    fn open(&mut self, path: Path) -> Result<i64, FsError> {
        panic!("Not implemented");
    }

    fn delete(&mut self, path: Path) -> FsResult<()> {
        panic!("Not implemented");
    }

    fn clear(&mut self, path: Path) -> FsResult<()> {
        panic!("Not implemented");
    }

    fn create_file(&mut self, path: Path) -> FsResult<()> {
        panic!("Not implemented");
    }

    fn create_directory(&mut self, path: Path) -> FsResult<()> {
        if self.exists_directory(path.clone())? {
            return Ok(());
        }

        match path.parent_dir() {
            None => { return Ok(()); },
            Some(p) => {
                self.create_directory(p.clone())?;
                let node_addr = self.node_addr_from_path(p)?;
                let node = self.read_node(node_addr)?;
                if node.node_type == NodeType::Directory {
                    let data = self.read_node_content(node)?;
                    let mut data = DirectoryData::from_blocks(&node, &data);
                    let addr = self.create_node(node_addr)?;

                    // create node
                    let mut dir = Node::null();
                    dir.node_type = NodeType::Directory;

                    let empty_dir_data = DirectoryData::empty();
                    // TODO create '.' and '..' links
                    self.write_node_content(addr, dir, empty_dir_data.to_blocks())?;

                    data.entries.push(DirectoryEntry {
                        name: path.name().unwrap(),
                        addr,
                    });

                    self.write_node_content(node_addr, node, data.to_blocks())?;

                    Ok(())
                } else {
                    Err(FsError::FileNotFound)
                }
            },
        }
    }

    fn exists_file(&mut self, path: Path) -> FsResult<bool> {
        let node = self.node_from_path(path);
        match node {
            Err(_) => Ok(false),
            Ok(x) => Ok(x.node_type == NodeType::File),
        }
    }

    fn exists_directory(&mut self, path: Path) -> FsResult<bool> {
        let node = self.node_from_path(path);
        match node {
            Err(_) => Ok(false),
            Ok(x) => Ok(x.node_type == NodeType::Directory),
        }
    }
}


#[test_case]
fn test_fffs_struct_sizes () {
    use core::mem::size_of;
    use crate::{serial_print, serial_println};
    serial_print!("test_fffs_struct_sizes... ");

    // all block structs should have size `BLOCK_SIZE`
    assert_eq!(size_of::<SuperBlock>(), BLOCK_SIZE);
    assert_eq!(size_of::<BlockGroupDescriptorBlock>(), BLOCK_SIZE);
    assert_eq!(size_of::<PointerData>(), BLOCK_SIZE);
    assert_eq!(size_of::<Block>(), BLOCK_SIZE);
    assert_eq!(size_of::<PointerData>(), BLOCK_SIZE);

    assert_eq!(size_of::<Node>(), 128);
    assert_eq!(size_of::<NodeType>(), 1);

    assert_eq!(size_of::<BlockGroupDescriptor>(), 32);

    assert_eq!(size_of::<RawAddr>(), 8);
    assert_eq!(size_of::<BlockAddr>(), 8);
    assert_eq!(size_of::<NodeAddr>(), 8);

    assert_eq!(size_of::<Permission>(), 2);

    serial_println!("[ok]");
}

