//! first fucking file system

use core::default::Default;
use crate::fs::{self, *, Path, AsU8Slice, FsResult, FsError, BlockDevice, SerdeBlockDevice};
use alloc::{vec, vec::Vec};
use crate::{println};


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
pub struct FileSystem<B: BlockDevice> {
    device: B,
    superblock: SuperBlock,
}

impl<B: BlockDevice> FileSystem<B> {
    /// creates a new fs on the given `BlockDevice`.
    pub fn format(mut device: B, name: &[u8]) -> FsResult<FileSystem<B>> {
        // copy partition name
        let mut part_name = [0u8; VOLUME_NAME_LEN];
        for (i, b) in name.iter().enumerate() {
            if i >= VOLUME_NAME_LEN {
                break;
            }
            part_name[i] = *b;
        }

        // create superblock
        let superblock = SuperBlock::new(device.blocks(), part_name);
        device.write(superblock_addr(), &superblock)?;

        let mut file_system = Self {
            device,
            superblock,
        };

        // initialize block groups with their respective descriptors
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
        self.superblock.block_group_count() / BGD_PER_BLOCK as u64 + 1
    }

    /// Initializes the block group descriptor table and all the block groups
    fn init_block_groups(&mut self) -> FsResult<()> {
        // create the entries for all descriptors

        for i in 0..self.bgdt_block_count() {
            let mut bgdt = [BlockGroupDescriptor::default(); BGD_PER_BLOCK];
            for k in 0..BGD_PER_BLOCK {
                let n = i * BGD_PER_BLOCK as u64 + k as u64;
                if n >= self.superblock.block_group_count() {
                    break;
                }
                bgdt[k] = self.create_bg_desc(n)?;
            }
            let addr = gdt_addr().offset(i as _);
            self.device.write(addr, &bgdt)?;
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
        let empty_block = [0u8; BLOCK_SIZE];

        let descriptor = BlockGroupDescriptor::new(
            RawAddr::new(group_offset),
            supr.usable_blocks_per_group(),
            supr.block_group_node_count as u16,
            0,
            );

        // override node usage table
        self.device.write(descriptor.node_usage_address(), &empty_block)?;

        let mut reserved_block_bitmap = empty_block;
        for (i, _node) in descriptor.node_blocks_begin().until(descriptor.node_blocks_end(&supr)).enumerate() {
            set_bit(&mut reserved_block_bitmap, 2 + i as usize, true);
            // not needed to clear the node
        }

        // overwrite block usage buffer to reserve blocks for the node table
        self.device.write(descriptor.node_usage_address(), &reserved_block_bitmap)?;

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
            let descriptor_block = gdt_addr().offset(descriptor_block as _);

            // read descriptor table
            let mut desc_table = [BlockGroupDescriptor::default(); BGD_PER_BLOCK];
            self.device.read(descriptor_block, &mut desc_table)?;

            Ok((desc_table, descriptor_block.as_u64(), descriptor_index as usize))
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
            Err(FsError::InvalidAddress)
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

        let block_index = descriptor.node_blocks_begin().offset(block as i64);

        // read the node table
        let mut node_table = node_table();
        self.device.read(block_index, &mut node_table)?;

        Ok((node_table, block_index.as_u64(), index as usize))
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

    /// deallocates all blocks used by the node
    fn clear_node_content(&mut self, node: &mut Node) -> FsResult<()> {
        let depth = node.indirection_level as usize + 1;
        let mut stack = Vec::with_capacity(depth);

        stack.push((0, node.pointers.to_vec()));

        while !stack.is_empty() {
            let addr = stack.last().unwrap().1[stack.last().unwrap().0];
            if !addr.is_null() {
                if stack.len() < depth {
                    // push pointer block
                    let addr = self.translate_block_addr(addr)?;
                    let mut pointer_data = PointerData::empty();
                    self.device.read(addr, &mut pointer_data)?;
                    stack.push((0, pointer_data.pointers.to_vec()));
                    continue;
                } else {
                    // deallocate content block
                    self.deallocate_block(addr)?;
                }
            }
            // increase index after deleting a block or when a pointer is null
            while !stack.is_empty() {
                let (index, table) = stack.pop().unwrap();
                let index = index + 1;
                if index < table.len() {
                    stack.push((index, table));
                } else {
                    match stack.pop() {
                        Some((i, t)) => {
                            // deallocate pointer block
                            let addr = t[i];
                            self.deallocate_block(addr)?;
                            stack.push((i, t));
                        },
                        None => {}, // there is no parent, can't deallocate pointers in the node
                    }
                }
            }
        }

        // clear all the pointers in the node
        node.size = 0;
        for addr in node.pointers.iter_mut() {
            *addr = BlockAddr::null();
        }
        Ok(())
    }

    /// writes the nodes contents blocks to the device
    fn write_node_content(&mut self, addr: NodeAddr, node: Node, content: Vec<Block>) -> FsResult<()> {
        let group = addr.inner_u64() / NODES_PER_GROUP;

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

        self.clear_node_content(&mut node)?;

        // write content size and indirection level to node
        node.size = (BLOCK_SIZE * content.len()) as _;
        node.indirection_level = indirection_level;

        let depth = indirection_level as usize;
        let mut stack = Vec::with_capacity(depth);

        stack.push((0, node.pointers.to_vec()));

        let mut content = content.iter();

        'outer: while !stack.is_empty() {
            // create a new pointer table if `depth` is not reached, otherwise write a block of content
            if stack.len() < depth {
                // add pointer block
                stack.push((0, PointerData::empty().pointers.to_vec()));
            } else {
                match content.next() {
                    Some(block) => {
                        // write block
                        let block_addr = self.allocate_block(group)?;
                        let raw_addr = self.translate_block_addr(block_addr)?;
                        self.device.write(raw_addr, &block)?;

                        // update pointer table
                        let (index, mut table) = stack.pop().unwrap();
                        table[index] = block_addr;
                        stack.push((index, table));

                        // increase the index
                        while !stack.is_empty() {
                            let (mut index, table) = stack.pop().unwrap();
                            index += 1;
                            if index >= table.len() {
                                // pointer table is full, need to write to disk and update parent table

                                if stack.len() == 0 {
                                    // pointer list in node
                                    if let Some(_) = content.next() {
                                        // pointer table is full, but there is still content
                                        return Err(FsError::InternalError);
                                    } else {
                                        // push table back to stack to write it to the node after the loop
                                        stack.push((index, table));
                                        break 'outer;
                                    }
                                } else {
                                    // there definitely is a parent table

                                    // write table to device
                                    let block_addr = self.allocate_block(group)?;
                                    let raw_addr = self.translate_block_addr(block_addr)?;
                                    self.device.write(raw_addr, &table)?;

                                    // update parent pointer table
                                    let (parent_index, mut parent_table) = stack.pop().unwrap();
                                    parent_table[parent_index] = block_addr;
                                    stack.push((parent_index, parent_table));
                                }
                            } else {
                                // don't write table to device, as there are still values to be written to the table
                                stack.push((index, table));
                            }
                        }
                    },
                    None => {
                        // write all tables except the table in the node
                        while stack.len() > 1 {
                            // write table
                            let (_, table) = stack.pop().unwrap();
                            let block_addr = self.allocate_block(group)?;
                            let raw_addr = self.translate_block_addr(block_addr)?;
                            self.device.write(raw_addr, &table)?;

                            // update parent table
                            let (index, mut parent_table) = stack.pop().unwrap();
                            parent_table[index] = block_addr;
                            stack.push((index, parent_table));
                        }
                        break;
                    },
                };
            }
        }

        // write node data
        let (_, table) = stack.pop().unwrap();
        copy(&table[..], &mut node.pointers, table.len());
        self.write_node(addr, node)?;
        Ok(())
    }

    /// returns the `NodeAddr` from the given path and returns errors if the node doesn't exist
    fn node_addr_from_path (&mut self, path: Path) -> FsResult<NodeAddr> {
        let mut addr = NodeAddr::root();
        for p in path.path.iter() {
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
        println!("{:#?}", descriptor);
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

            // if there is no free block, but the group descriptor is marked to have free blocks, that's a bug
            panic!("Block unused node count doesn't correspond to node usage table");
        }
        Err(FsError::InternalError)
    }

    /// fills the block given by `addr` with zeroes
    fn clear_block(&mut self, addr: RawAddr) -> FsResult<()> {
        let empty = [0u8; BLOCK_SIZE];
        self.device.write(addr, &empty)?;
        Ok(())
    }

    /// frees `block` from the file system
    fn deallocate_block(&mut self, block: BlockAddr) -> FsResult<()> {
        // find out which group the block is in
        let (group, index) = self.block_addr_to_group_index(block);
        let index = index as usize;

        // read descriptor table
        let (mut desc_table, table_block, desc_index) = self.group_descriptor_table(group)?;
        let mut descriptor = desc_table[desc_index];

        // clear the block
        let addr = descriptor.usable_blocks_begin(&self.superblock).offset(index as _);
        self.clear_block(addr)?;

        // read usage table
        let mut usage_table = [0u8; BLOCK_SIZE];
        self.device.read(descriptor.block_usage_address(), &mut usage_table)?;

        if get_bit(&usage_table, index) {
            // mark block as unused and write usage table and group descriptor

            // write usage table
            set_bit(&mut usage_table, index, false);
            self.device.write(descriptor.block_usage_address(), &usage_table)?;

            // write descriptor
            descriptor.unused_blocks += 1;
            desc_table[desc_index] = descriptor;
            self.device.write(table_block, &desc_table)?;

            Ok(())
        } else {
            // block is already unused
            Err(FsError::InvalidAddress)
        }
    }

    fn delete_empty_node(&mut self, addr: NodeAddr) -> FsResult<()> {
        let node = self.read_node(addr)?;
        if node.size > 0 {
            panic!("Node still has data");
        } else {
            let group = addr.inner_u64() / NODES_PER_GROUP;
            let node_index = addr.inner_u64() % NODES_PER_GROUP;

            // load descriptor table
            let (mut desc_table, block_index, table_index) = self.group_descriptor_table(group)?;
            let mut descriptor = desc_table[table_index];

            // update usage table
            let mut usage_table = [0u8; BLOCK_SIZE];
            self.device.read(descriptor.node_usage_address(), &mut usage_table)?;
            set_bit(&mut usage_table, node_index as usize, false);
            self.device.write(descriptor.node_usage_address(), &usage_table)?;

            // update unused nodes count
            descriptor.unused_nodes += 1;
            desc_table[table_index] = descriptor;
            self.device.write(block_index, &desc_table)?;

            Ok(())
        }
    }

    fn delete_file(&mut self, file: NodeAddr) -> FsResult<()> {
        self.clear_file(file)?;
        self.delete_empty_node(file)?;
        Ok(())
    }

    fn delete_directory(&mut self, dir: NodeAddr) -> FsResult<()> {
        self.clear_directory(dir)?;
        self.delete_empty_node(dir)?;
        Ok(())
    }

    fn clear_file(&mut self, file: NodeAddr) -> FsResult<()> {
        let mut node = self.read_node(file)?;
        if node.node_type != NodeType::File {
            panic!("Node is not a file");
        }
        self.clear_node_content(&mut node)?;
        self.write_node(file, node)?;
        Ok(())
    }

    fn clear_directory(&mut self, dir: NodeAddr) -> FsResult<()> {
        let mut node = self.read_node(dir)?;
        if node.node_type != NodeType::Directory {
            panic!("Node is not a directory");
        }
        let content = self.read_node_content(node)?;
        let content = DirectoryData::from_blocks(&node, &content);

        // delete all directory entries
        for entry in content.entries.iter() {
            let node = self.read_node(entry.addr)?;
            match node.node_type {
                NodeType::File => self.delete_file(entry.addr)?,
                NodeType::Directory => self.delete_directory(entry.addr)?,
                _ => panic!("Not supported"),
            };
        }

        // clear node content
        self.clear_node_content(&mut node)?;
        self.write_node(dir, node)?;

        Ok(())
    }
}

// TODO
impl<B: BlockDevice> fs::FileSystem<B> for FileSystem<B> {
    fn allowed_chars() -> &'static [u8] {
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-."
    }

    fn separator() -> u8 {
        b'/'
    }

    /// takes the given blockdevice and tries to read it as this fs
    fn mount(device: B) -> FsResult<FileSystem<B>> {
        let mut device = device;
        let mut superblock = SuperBlock::empty();
        device.read(superblock_addr(), &mut superblock)?;
        if superblock.is_valid() {
            superblock.mark_mounted();
            device.write(superblock_addr(), &mut superblock)?;
            let file_system = Self {
                device,
                superblock,
            };
            Ok(file_system)
        } else {
            Err(FsError::InvalidSuperBlock)
        }
    }

    fn open(&mut self, _path: Path) -> Result<i64, FsError> {
        panic!("Not implemented");
    }

    fn delete(&mut self, path: Path) -> FsResult<()> {
        self.clear(path.clone())?;
        let addr = self.node_addr_from_path(path)?;
        self.delete_empty_node(addr)?;
        Ok(())
    }

    fn clear(&mut self, path: Path) -> FsResult<()> {
        let addr = self.node_addr_from_path(path)?;
        let node = self.read_node(addr)?;
        match node.node_type {
            NodeType::File => self.clear_file(addr),
            NodeType::Directory => self.clear_directory(addr),
            _ => panic!("Not implemented")
        }
    }

    fn create_file(&mut self, path: Path) -> FsResult<()> {
        if !self.exists_file(path.clone())? && !self.exists_directory(path.clone())? {
            match path.parent_dir() {
                None => Err(FsError::IllegalOperation), // root node is reserved for root directory
                Some(p) => {
                    self.create_directory(p.clone())?;

                    // read parent node
                    let parent_node_addr = self.node_addr_from_path(p)?;
                    let parent_node = self.read_node(parent_node_addr)?;

                    if parent_node.node_type == NodeType::Directory {
                        // read parent directory content
                        let data = self.read_node_content(parent_node)?;
                        let mut data = DirectoryData::from_blocks(&parent_node, &data);

                        // create file node
                        let addr = self.create_node(parent_node_addr)?;
                        let file_node = Node::null();
                        let empty_file_data = FileData::empty();
                        self.write_node_content(addr, file_node, empty_file_data.to_blocks())?;

                        // write file entry to parent dir
                        data.entries.push(DirectoryEntry {
                            name: path.name().unwrap(),
                            addr,
                        });
                        self.write_node_content(parent_node_addr, parent_node, data.to_blocks())?;

                        Ok(())
                    } else {
                        Err(FsError::IllegalOperation) // parent node is not a directory
                    }
                }
            }
        } else {
            Err(FsError::IllegalOperation) // cant create a file when a file or directory with that name already exists
        }
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

