use super::*;

// random number
const MAGIC: u64 = 9015850442860206877;

#[repr(C)]
#[repr(align(4096))]
pub struct RootBlock {
    magic: u64,
    root_node: u64,
    free_blocks: u64,
    name: [u8; 256],
}

impl RootBlock {
    pub fn new () -> Self {
        Self {
            magic: MAGIC,
            root_node: 0,
            free_blocks: 0,
            name: [b'a'; 256],
        }
    }
}

#[repr(u8)]
pub enum NodeType {
    Directory,
    File,
    Link,
}

const DATA_LEN: usize = 3072;

/// how the data of the node is stored
#[repr(C)]
pub enum NodeData {

    /// File, stored in the node
    File([u8; DATA_LEN]),

    /// Directory, stored in the node
    Dir([u64; DATA_LEN / 8]),

    /// File or directory, indirectly stored in variable levels of indirection
    Indir((u8, [u64; DATA_LEN / 8])),
    //      ^   ^^^^^^^^^^^^^^^^^^  -- links to data
    //      |
    //      +-- level of indirection
}

#[repr(C)]
pub struct Owner {
    // TODO
}

/// Memory representation of a file, directory or symbolic link
#[repr(C)]
#[repr(align(4096))]
pub struct Node {
    pub name: [u8; 256],
    pub owner: Owner,
    pub node_type: NodeType,
    pub size: u64,
    pub data: NodeData,
}

/// Memory representation of a link block that either links to other link blocks or data blocks
#[repr(C)]
#[repr(align(4096))]
pub struct Links {
    pub links: [u64; 512],
}

#[test_case]
fn test_rfs_structs_memory_layout() {
    use core::mem::size_of;
    use crate::{serial_print, serial_println};
    serial_print!("test_rfs_structs_memory_layout... ");
    // make sure that all blocks have size 4096
    assert_eq!(size_of::<RootBlock>(), 4096);
    assert_eq!(size_of::<Node>(), 4096);
    assert_eq!(size_of::<Links>(), 4096);
    assert_eq!(size_of::<NodeType>(), 1);
    serial_println!("[ok]");
}





