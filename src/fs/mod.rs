use core::mem::{size_of, transmute};
use core::ptr::Unique;
use core::marker::PhantomData;

pub mod rfs;
pub mod fffs;
pub mod bs;

use bs::BlockSize;

/// Error type for file system errors
pub enum FsError {
    /// invalid block index
    InvalidIndex,

    /// struct / buffer size does not correspond to block size
    MalformedBuffer,

    /// root block is corrupted
    InvalidRootBlock,

    /// file or directory does not exist
    FileNotFound,

    /// no permission to access path
    AccessViolation,

    /// SuperBlock invalid or not found
    InvalidSuperBlock,
}

/// Result type for file system operations
pub type FsResult<T> = Result<T, FsError>;

/// Generic device that can be read from or written to on a block by block basis
pub trait BlockDevice<S: BlockSize<BS>, const BS: usize> {

    /// Number of blocks on the device
    fn blocks(&self) -> u64;

    /// Basic read operation:
    /// reads block from `self` into `buffer`
    fn read_block(&mut self, index: u64, buffer: &mut [u8; BS]) -> FsResult<()>;

    /// Basic write operation:
    /// writes `buffer` to `self`
    fn write_block(&mut self, index: u64, buffer: &[u8; BS]) -> FsResult<()>;
}

/// This trait allows `Sized` types to be read and written to and from the block device.
/// It uses unsafe memory access to read the bytes of an instance of `T` or write to the bytes of an instance of `T`.
/// It is useful if the generic type `T` is `repr(C)` and `repr(align(BS)).
pub trait SerdeBlockDevice<T: Sized, S: BlockSize<BS>, const BS: usize> {
    /// Tries to read from `self` and write the raw bytes to `obj` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn read(&mut self, index: u64, obj: &mut T) -> FsResult<()>;

    /// Tries to read from `obj` and write the raw bytes to `self` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn write(&mut self, index: u64, obj: &mut T) -> FsResult<()>;
}

/// Simple trait that checks if the I/O operation is valid based on limited information about the block device
trait BlockDeviceArgumentChecks<S: BlockSize<BS>, const BS: usize> {
    fn check_args(&self, index: u64) -> FsResult<()>;
}

impl<T: BlockDevice<S, BS>, S: BlockSize<BS>, const BS: usize> BlockDeviceArgumentChecks<S, BS> for T {
    fn check_args(&self, index: u64) -> FsResult<()> {
        if index >= self.blocks() {
            Err(FsError::InvalidIndex)
        } else {
            Ok(())
        }
    }
}

impl<T: Sized, S: BlockSize<BS>, B: BlockDevice<S, BS>, const BS: usize> SerdeBlockDevice<T, S, BS> for B {
    fn read(&mut self, index: u64, obj: &mut T) -> FsResult<()> {
        if size_of::<T>() != BS {
            Err(FsError::MalformedBuffer)
        } else {
            Ok(self.read_block(index, unsafe{
                transmute::<&mut T, &mut [u8; BS]>(obj)
            })?)
        }
    }

    fn write(&mut self, index: u64, obj: &mut T) -> FsResult<()> {
        if core::mem::size_of::<T>() != BS {
            Err(FsError::MalformedBuffer)
        } else {
            Ok(self.write_block(index, unsafe {
                core::mem::transmute::<&mut T, &mut [u8; BS]>(obj)
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

impl BlockDevice<bs::Size4KiB, 4096> for RamDisk {

    fn blocks(&self) -> u64 {
        self.block_count
    }

    fn read_block(&mut self, index: u64, buffer: &mut [u8; RAM_BS]) -> FsResult<()> {
        self.check_args(index)?;
        let block = self.block_slice(index);
        copy(block, buffer, RAM_BS);
        Ok(())
    }

    fn write_block(&mut self, index: u64, buffer: &[u8; RAM_BS]) -> FsResult<()> {
        self.check_args(index)?;
        let block = self.block_slice(index);
        copy(buffer, block, RAM_BS);
        Ok(())
    }
}

pub struct Path;

pub trait FileSystem<S: BlockSize<BS>, const BS: usize> : Sized {
    /// Struct representing root block
    type RootBlock: Sized;

    /// Struct representing file system node, e.g. a directory or a file
    type Node: Sized;

    /// allowed characters in file names
    fn allowed_chars() -> &'static str;

    /// separates directory names
    fn separator() -> &'static str;

    /// checks if the root block is valid
    fn is_valid_root_block(root_block: &Self::RootBlock) -> bool;

    /// creates a new file system using the `block_device` or fails if the root block is not valid
    fn new(block_device: &mut impl BlockDevice<S, BS>, root_block: Self::RootBlock) -> Result<Self, FsError>;

    /// opens a file / directory and returns a file descriptor
    fn open(path: Path) -> Result<i64, FsError>;

    /// deletes a file or directory
    fn delete(path: Path) -> FsResult<()>;

    /// clears the file, but does not delete it
    fn clear(path: Path) -> FsResult<()>;

    /// creates a new file at the path
    fn create_file(path: Path) -> FsResult<()>;

    /// creates a new directory at the path
    fn create_directory(path: Path) -> FsResult<()>;

}

pub trait Encryption {
    fn encrypt(&self, src: &[u8], dest: &mut [u8]);
    fn decrypt(&self, src: &[u8], dest: &mut [u8]);
}

/// `BlockDevice` that is being encrypted on each read and write using `Encryption`.
pub struct EncryptedBlockDevice<E: Encryption, B: BlockDevice<S, BS>, S: BlockSize<BS>, const BS: usize> {
    encryption: E,
    device: B,
    _phantom: PhantomData<S>,
}

impl<E: Encryption, B: BlockDevice<S, BS>, S: BlockSize<BS>, const BS: usize>  BlockDevice<S, BS> for EncryptedBlockDevice<E, B, S, BS> {
    fn blocks(&self) -> u64 {
        self.device.blocks()
    }
    fn read_block(&mut self, index: u64, buffer: &mut [u8; BS]) -> FsResult<()> {
        let mut tmp = S::mem_block();
        self.device.read_block(index, &mut tmp)?;
        self.encryption.decrypt(&tmp, buffer);
        Ok(())
    }

    fn write_block(&mut self, index: u64, buffer: &[u8; BS]) -> FsResult<()> {
        let mut tmp = S::mem_block();
        self.encryption.encrypt(&buffer[..], &mut tmp[..]);
        self.device.write_block(index, &tmp)
    }
}

