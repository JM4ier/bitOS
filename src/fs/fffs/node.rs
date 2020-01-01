use alloc::vec::Vec;
use super::{*, block::*, pointer::*};
use crate::fs::*;

use core::cmp::min;
use alloc::boxed::Box;

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum NodeType {
    Directory,
    File,
    SymLink,
}

#[derive(Copy, Clone)]
#[repr(C, align(128))]
pub struct Node {
    /// the type of the node
    pub node_type: NodeType,

    /// posix-like permissions (rwxrwxrwx)
    pub permission: Permission,

    /// user id of user owning the file
    pub user: u16,

    /// group id of group owning the file
    pub group: u16,

    /// size of the data associated with the node
    /// (file size if it is a file, size of directory info if it is a directory)
    pub size: u64,

    /// time of creation
    pub created: Time,

    /// last time the node was accessed
    pub last_access: Time,

    /// last time the node was modified
    pub last_modified: Time,

    /// time the node was deleted, if it is deleted
    pub deleted: Time,

    /// levels of indirection to the data
    pub indirection_level: u64,

    /// potentially indirect pointers to data
    pub pointers: [BlockAddr; 9],
}

impl Node {
    fn null() -> Self {
        Self {
            node_type: NodeType::File,
            permission: Permission::default(),
            user: 0,
            group: 0,
            size: 0,
            created: 0,
            last_access: 0,
            last_modified: 0,
            deleted: 0,
            indirection_level: 0,
            pointers: [BlockAddr::null(); 9],
        }
    }
}

pub fn node_table () -> [Node; BLOCK_SIZE / 128] {
    [Node::null(); BLOCK_SIZE / 128]
}

pub trait NodeData {
    fn to_blocks(&self) -> Vec<Block>;
    fn from_blocks(node: &Node, blocks: &Vec<Block>) -> Self;
}

pub struct DirectoryData {
    pub entries: Vec<DirectoryEntry>,
}

pub struct DirectoryEntry {
    pub name: Vec<u8>,
    pub addr: NodeAddr,
}

impl DirectoryEntry {
    fn size(&self) -> usize {
        // name length, address, name
        1 + 8 + self.name.len()
    }
}

impl NodeData for DirectoryData {
    fn to_blocks(&self) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut index = 0;

        while index < self.entries.len() {
            let mut selected = Vec::new();
            let size = 8; // size in bytes, init with 4 because of length of list of entries

            // select as many entries as possible to put in one block
            while index < self.entries.len() {
                if size + self.entries[index].size() <= BLOCK_SIZE {
                    selected.push(&self.entries[index]);
                    index += 1;
                } else {
                    break;
                }
            }
            let mut block = [0u8; BLOCK_SIZE];
            let mut offset = 0;

            // copy amount of entries into block
            copy_offset((selected.len() as u64).as_u8_slice(), &mut block, 8, 0, offset);
            offset += 8;

            // copy all the addresses first so they are aligned
            for s in selected.iter() {
                copy_offset(s.addr.as_u8_slice(), &mut block, 8, 0, offset);
                offset += 8;
            }

            // copy entry names with corresponding sizes
            for s in selected.iter() {
                // copy length
                copy_offset((s.name.len() as u8).as_u8_slice(), &mut block, 1, 0, offset);
                offset += 1;

                // copy name
                copy_offset(&s.name, &mut block, s.name.len(), 0, offset);
                offset += s.name.len();
            }

            blocks.push(block);
        }

        blocks
    }

    fn from_blocks(_node: &Node, blocks: &Vec<Block>) -> Self {
        let mut entries = Vec::new();

        for block in blocks {
            let mut offset = 0;
            let mut size = 0u64;
            copy_offset(block, &mut size.as_u8_slice_mut(), 8, offset, 0);
            offset += 8;
            let size = size as usize;

            // read the entries of this block
            for i in 0..size {
                // read length of name
                let mut size = 0u8;
                copy_offset(block, &mut size.as_u8_slice_mut(), 1, offset, 0);
                offset += 1;
                let size = size as usize;

                // read name
                let mut name = vec![0; size];
                copy_offset(block, &mut name, size, offset, 0);
                offset += size;

                // read address
                let mut addr = NodeAddr::null();
                copy_offset(block, &mut addr.as_u8_slice_mut(), 8, (i+1) * 8, 0);

                // add to entries
                entries.push(
                    DirectoryEntry {
                        addr,
                        name,
                    }
                );
            }
        }

        Self {
            entries
        }
    }
}

pub struct FileData {
    pub data: Vec<u8>,
}

impl NodeData for FileData {
    fn to_blocks(&self) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut offset = 0;
        while offset < self.data.len() {
            let size = min(self.data.len() - offset, BLOCK_SIZE);
            let mut block = [0u8; BLOCK_SIZE];
            copy(&self.data[offset..(offset + size)], &mut block, size);
            blocks.push(block);
            offset += size;
        }
        blocks
    }

    fn from_blocks(node: &Node, blocks: &Vec<Block>) -> Self {
        let mut data = Vec::with_capacity(node.size as usize);
        for i in 0..blocks.len() {
            let offset = BLOCK_SIZE * i;
            let size = min(BLOCK_SIZE, data.len() - offset);
            copy(&blocks[i][..], &mut data[offset..offset+size], size);
        }
        Self {
            data
        }
    }
}

