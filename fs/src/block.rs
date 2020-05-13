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

impl<D, const BS: usize> RWBlockDevice<BS> for D
where D: ReadBlockDevice<BS> + WriteBlockDevice<BS>
{}

pub fn check_args<D: BlockDevice<BS>, const BS: usize>(device: &D, buffer: &[u8], index: usize) -> FsResult<()> {
    if buffer.len() != BS {
        Err(FsError::InternalError("invalid buffer size".to_string()))
    } else if index > device.blocks() {
        Err(FsError::InternalError("out of bounds block address".to_string()))
    } else {
        Ok(())
    }
}

/// Quality-of-life trait that trasmutes any `T` to a byte buffer and reads a block
pub trait TransmutingReadBlockDevice<const BS: usize> {
    fn read<T>(&self, index: usize, buffer: &mut T) -> FsResult<()>;
}

/// Quality-of-life trait that trasmutes any `T` to a byte buffer and writes a block
pub trait TransmutingWriteBlockDevice<const BS: usize> {
    fn write<T>(&mut self, index: usize, buffer: &T) -> FsResult<()>;
}

impl<D: ReadBlockDevice<BS>, const BS: usize> TransmutingReadBlockDevice<BS> for D {
    fn read<T>(&self, index: usize, buffer: &mut T) -> FsResult<()> {
        let size = core::mem::size_of::<T>();
        let buffer = unsafe {
            &mut *core::ptr::slice_from_raw_parts_mut((buffer as *mut T) as *mut u8, size)
        };
        self.read_block(index, buffer)
    }
}

impl<D: WriteBlockDevice<BS>, const BS: usize> TransmutingWriteBlockDevice<BS> for D {
    fn write<T>(&mut self, index: usize, buffer: &T) -> FsResult<()> {
        let size = core::mem::size_of::<T>();
        let buffer = unsafe {
            &*core::ptr::slice_from_raw_parts((buffer as *const T) as *const u8, size)
        };
        self.write_block(index, buffer)
    }
}

