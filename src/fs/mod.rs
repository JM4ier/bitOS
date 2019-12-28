use core::mem::{size_of, transmute};
use core::ptr::Unique;

/// Error type for file system errors
pub enum FsError {
    InvalidIndex,
    MalformedBuffer,
}

/// Result type for file system operations
pub type FsResult = Result<(), FsError>;

/// Generic device that can be read from or written to on a block by block basis
pub trait BlockDevice<const BLOCK_SIZE: usize> {

    /// Number of blocks on the device
    fn blocks(&self) -> u64;

    /// Basic read operation:
    /// reads block from `self` into `buffer`
    fn read_block(&mut self, index: u64, buffer: &mut [u8; BLOCK_SIZE]) -> FsResult;

    /// Basic write operation:
    /// writes `buffer` to `self`
    fn write_block(&mut self, index: u64, buffer: &[u8; BLOCK_SIZE]) -> FsResult;
}

/// This trait allows `Sized` types to be read and written to and from the block device.
/// It uses unsafe memory access to read the bytes of an instance of `T` or write to the bytes of an instance of `T`.
/// It is useful if the generic type `T` is `repr(C)`.
pub trait SerdeBlockDevice<T: Sized, const BLOCK_SIZE: usize> {
    /// Tries to read from `self` and write the raw bytes to `obj` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn read(&mut self, index: u64, obj: &mut T) -> FsResult;

    /// Tries to read from `obj` and write the raw bytes to `self` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn write(&mut self, index: u64, obj: &mut T) -> FsResult;
}

/// Simple trait that checks if the I/O operation is valid based on limited information about the block device
trait BlockDeviceArgumentChecks<const BLOCK_SIZE: usize> {
    fn check_args(&self, index: u64) -> FsResult;
}

impl<T: BlockDevice<BLOCK_SIZE>, const BLOCK_SIZE: usize> BlockDeviceArgumentChecks<BLOCK_SIZE> for T {
    fn check_args(&self, index: u64) -> FsResult {
        if index >= self.blocks() {
            Err(FsError::InvalidIndex)
        } else {
            Ok(())
        }
    }
}

impl<T: Sized, B: BlockDevice<BLOCK_SIZE>, const BLOCK_SIZE: usize> SerdeBlockDevice<T, BLOCK_SIZE> for B {
    fn read(&mut self, index: u64, obj: &mut T) -> FsResult {
        if size_of::<T>() != BLOCK_SIZE {
            Err(FsError::MalformedBuffer)
        } else {
            Ok(self.read_block(index, unsafe{
                transmute::<&mut T, &mut [u8; BLOCK_SIZE]>(obj)
            })?)
        }
    }

    fn write(&mut self, index: u64, obj: &mut T) -> FsResult {
        if core::mem::size_of::<T>() != BLOCK_SIZE {
            Err(FsError::MalformedBuffer)
        } else {
            Ok(self.write_block(index, unsafe {
                core::mem::transmute::<&mut T, &mut [u8; BLOCK_SIZE]>(obj)
            })?)
        }
    }
}

/// RAM disk block size
const RAM_BS: usize = 4096;

/// Simple disk that stores the data in memory
pub struct RamDisk {
    block_count: u64,
    disk_begin: Unique<[u8; RAM_BS]>,
}

impl RamDisk {
    pub unsafe fn new (addr: u64, blocks: u64) -> Self {
        Self {
            block_count: blocks,
            disk_begin: Unique::new_unchecked(addr as _),
        }
    }

    fn block_slice(&mut self, index: u64) -> &mut [u8] {
        unsafe {
            let block_ptr = self.disk_begin.as_ptr();
            let offset = RAM_BS as isize + index as isize;
            &mut *block_ptr.offset(offset)
        }
    }
}

fn copy<T: Copy>(input: &[T], output: &mut [T], size: usize) {
    for i in 0..size{
        output[i] = input[i];
    }
}

impl BlockDevice<RAM_BS> for RamDisk {

    fn blocks(&self) -> u64 {
        self.block_count
    }

    fn read_block(&mut self, index: u64, buffer: &mut [u8; RAM_BS]) -> FsResult {
        self.check_args(index)?;
        let block = self.block_slice(index);
        copy(block, buffer, RAM_BS);
        Ok(())
    }

    fn write_block(&mut self, index: u64, buffer: &[u8; RAM_BS]) -> FsResult {
        self.check_args(index)?;
        let block = self.block_slice(index);
        copy(buffer, block, RAM_BS);
        Ok(())
    }
}

