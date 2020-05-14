extern crate alloc;

use alloc::vec::*;

use crate::block::*;
use crate::copy::*;
use crate::error::*;

pub struct RamDisk<'a> {
    pub data: &'a mut [u8],
}

pub struct RomDisk<'a> {
    pub data: &'a [u8],
}

pub struct OwnedDisk {
    pub data: Vec<u8>,
}

macro_rules! impl_read_block {
    ($type: ty) => (
        impl BlockDevice for $type {
            fn block_size(&self) -> usize {
                4096
            }
            fn blocks(&self) -> usize {
                self.data.len() / self.block_size()
            }
        }
        impl ReadBlockDevice for $type {
            fn read_block(&self, index: usize, buffer: &mut [u8]) -> FsResult<()> {
                check_args::<$type>(self, buffer, index)?;
                let bs = self.block_size();
                let begin = bs * index;
                copy_offset(&self.data, buffer, bs, begin, 0);
                Ok(())
            }
        }
    )
}

macro_rules! impl_write_block {
    ($type: ty) => (
        impl WriteBlockDevice for $type {
            fn write_block(&mut self, index: usize, buffer: &[u8]) -> FsResult<()> {
                check_args::<$type>(self, buffer, index)?;
                let bs = self.block_size();
                let begin = bs * index;
                copy_offset(buffer, &mut self.data, bs, 0, begin);
                Ok(())
            }
        }
    )
}

impl_read_block!(RamDisk<'_>);
impl_read_block!(RomDisk<'_>);
impl_read_block!(OwnedDisk);

impl_write_block!(RamDisk<'_>);
impl_write_block!(OwnedDisk);

