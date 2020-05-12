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
        impl<const BS: usize> BlockDevice<BS> for $type {
            fn blocks(&self) -> usize {
                self.data.len() / BS
            }
        }
        impl<const BS: usize> ReadBlockDevice<BS> for $type {
            fn read_block(&self, index: usize, buffer: &mut [u8]) -> FsResult<()> {
                check_args::<$type, BS>(self, buffer, index)?;
                let begin = BS * index;
                copy_offset(&self.data, buffer, BS, begin, 0);
                Ok(())
            }
        }
    )
}

macro_rules! impl_write_block {
    ($type: ty) => (
        impl<const BS: usize> WriteBlockDevice<BS> for $type {
            fn write_block(&mut self, index: usize, buffer: &[u8]) -> FsResult<()> {
                check_args::<$type, BS>(self, buffer, index)?;
                let begin = BS * index;
                copy_offset(buffer, &mut self.data, BS, 0, begin);
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

