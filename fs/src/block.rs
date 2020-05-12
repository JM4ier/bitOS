#![feature(const_generics)]

extern crate alloc;
use alloc::string::*;
use crate::error::*;


/// Generic device that can be read from or written to on a block by block basis
pub trait BlockDevice<const BS: usize> {
    /// Number of blocks on the device
    fn blocks(&self) -> usize;
}

pub trait ReadBlockDevice<const BS: usize> : BlockDevice<BS> {
    /// Basic read operation:
    /// reads block from `self` into `buffer`
    fn read_block(&self, index: usize, buffer: &mut [u8]) -> FsResult<()>;
}

pub trait WriteBlockDevice<const BS: usize> : BlockDevice<BS> {
    /// Basic write operation:
    /// writes `buffer` to `self`
    fn write_block(&mut self, index: usize, buffer: &[u8]) -> FsResult<()>;
}

pub trait RWBlockDevice<const BS: usize> : ReadBlockDevice<BS> + WriteBlockDevice<BS> {}

pub fn check_args<D: BlockDevice<BS>, const BS: usize>(device: &D, buffer: &[u8], index: usize) -> FsResult<()> {
    if buffer.len() != BS {
        Err(FsError::InternalError("invalid buffer size".to_string()))
    } else if index > device.blocks() {
        Err(FsError::InternalError("out of bounds block address".to_string()))
    } else {
        Ok(())
    }
}

