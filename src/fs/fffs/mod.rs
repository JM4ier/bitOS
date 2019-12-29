//! first fucking file system

use core::cmp::PartialEq;
use core::marker::PhantomData;
use core::mem;
use core::default::Default;
use crate::fs::{self, FsResult, FsError, SerdeBlockDevice};

use self::super::bs::Size4KiB;

pub mod perm;
use self::perm::*;

pub mod superblock;
use self::superblock::*;

pub mod block;
use self::block::*;

pub mod node;
use self::node::*;

pub const BLOCK_SIZE: usize = 4096;
pub const BLOCK_GROUP_SIZE: u64 = 8192;
pub const NODES_PER_GROUP: u64 = 1536;

pub type Time = i64;

pub struct FileSystem<D>
where D: fs::BlockDevice<Size4KiB, BLOCK_SIZE>,
{
    device: D,
    superblock: SuperBlock,
}

impl<D> FileSystem<D>
where D: fs::BlockDevice<Size4KiB, BLOCK_SIZE>
{

    /// takes the given blockdevice and tries to read it as this fs
    pub fn mount(mut device: D) -> FsResult<FileSystem<D>> {
        let mut superblock = SuperBlock::empty();
        device.read(superblock_addr().as_u64(), &mut superblock)?;
        if superblock.is_valid() {
            superblock.mark_mounted();
            device.write(superblock_addr().as_u64(), &mut superblock)?;
            let file_system = Self {
                device,
                superblock,
                _phantom: PhantomData,
            };
            // TODO load bgdt and initialize stuff
            Ok(file_system)
        } else {
            Err(FsError::InvalidSuperBlock)
        }
    }

    /// creates a new fs on the given `BlockDevice`.
    pub fn format(mut device: D, name: &[u8]) -> FsResult<FileSystem<D>> {
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
            device,
            superblock,
            _phantom: PhantomData,
        };
        file_system.init_bgdt()?;
        Ok(file_system)
    }

    fn init_bgdt(&mut self) -> FsResult<()> {
        // number of block groups
        let block_group_count = self.superblock.blocks / self.superblock.block_group_size;

        // number of blocks the block group descriptor table needs
        let bgdt_block_count = block_group_count / BGD_PER_BLOCK as u64;

        // create the entries for
        for i in 0..bgdt_block_count {
            let mut bgdt = [BlockGroupDescriptor::default(); BGD_PER_BLOCK];
            for k in 0..BGD_PER_BLOCK {
                let n = i * BGD_PER_BLOCK as u64 + k as u64;
                bgdt[k] = self.create_bg_desc(n)?;
            }
        }
        Ok(())
    }

    fn create_bg_desc (&mut self, index: u64) -> FsResult<BlockGroupDescriptor> {
        let reserved_offset = self.superblock.reserved;
        let group_size = self.superblock.block_group_size;
        let group_offset = reserved_offset + index * group_size;
        let empty_block = [0; BLOCK_SIZE];

        for i in 0..3 {
            self.device.write_block(group_offset + i, &empty_block)?;
        }


        panic!("Unimplemented");
    }
}



#[test_case]
fn test_fffs_struct_sizes () {
    use core::mem::size_of;
    use crate::{serial_print, serial_println};
    serial_print!("test_fffs_struct_sizes... ");
    assert_eq!(size_of::<SuperBlock>(), 4096);
    assert_eq!(size_of::<BlockGroupDescriptorBlock>(), 4096);
    assert_eq!(size_of::<Node>(), 128);
    assert_eq!(size_of::<NodeType>(), 1);
    assert_eq!(size_of::<BlockGroupDescriptor>(), 32);
    assert_eq!(size_of::<RawBlockAddr>(), 8);
    assert_eq!(size_of::<BlockAddr>(), 8);
    assert_eq!(size_of::<Permission>(), 2);
    serial_println!("[ok]");
}
