use core::ptr::Unique;

pub mod fffs;

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
pub trait BlockDevice {
    /// blocksize in bytes
    fn blocksize(&self) -> u64;

    /// Number of blocks on the device
    fn blocks(&self) -> u64;

    /// Basic read operation:
    /// reads block from `self` into `buffer`
    fn read_block(&mut self, index: u64, buffer: &mut [u8]) -> FsResult<()>;

    /// Basic write operation:
    /// writes `buffer` to `self`
    fn write_block(&mut self, index: u64, buffer: &[u8]) -> FsResult<()>;
}

/// This trait allows `Sized` types to be read and written to and from the block device.
/// It uses unsafe memory access to read the bytes of an instance of `T` or write to the bytes of an instance of `T`.
/// It is useful if the generic type `T` is `repr(C)` and `repr(align(BS)).
pub trait SerdeBlockDevice<T> {
    /// Tries to read from `self` and write the raw bytes to `obj` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn read(&mut self, index: u64, obj: &mut T) -> FsResult<()>;

    /// Tries to read from `obj` and write the raw bytes to `self` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn write(&mut self, index: u64, obj: &mut T) -> FsResult<()>;
}

/// Simple trait that checks if the I/O operation is valid based on limited information about the block device
trait BlockDeviceArgumentChecks {
    fn check_args(&self, index: u64, buffer: &[u8]) -> FsResult<()>;
}

impl<D: BlockDevice> BlockDeviceArgumentChecks for D {
    fn check_args(&self, index: u64, buffer: &[u8]) -> FsResult<()> {
        if buffer.len() as u64 != self.blocksize() {
            Err(FsError::MalformedBuffer)
        } else if index >= self.blocks() {
            Err(FsError::InvalidIndex)
        } else {
            Ok(())
        }
    }
}

impl<T: Sized, B: BlockDevice> SerdeBlockDevice<T> for B {
    fn read(&mut self, index: u64, obj: &mut T) -> FsResult<()> {
        self.read_block(index, obj.as_u8_slice_mut())
    }

    fn write(&mut self, index: u64, obj: &mut T) -> FsResult<()> {
        self.write_block(index, obj.as_u8_slice())
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

impl BlockDevice for RamDisk {

    fn blocksize(&self) -> u64 {
        RAM_BS as u64
    }

    fn blocks(&self) -> u64 {
        self.block_count
    }

    fn read_block(&mut self, index: u64, buffer: &mut [u8]) -> FsResult<()> {
        self.check_args(index, buffer)?;
        let block = self.block_slice(index);
        copy(block, buffer, RAM_BS);
        Ok(())
    }

    fn write_block(&mut self, index: u64, buffer: &[u8]) -> FsResult<()> {
        self.check_args(index, buffer)?;
        let block = self.block_slice(index);
        copy(buffer, block, RAM_BS);
        Ok(())
    }
}

pub struct Path;

pub trait FileSystem : Sized  {
    /// allowed characters in file names
    fn allowed_chars() -> &'static [u8];

    /// separates directory names
    fn separator() -> u8;

    /// creates a new file system using the `block_device` or fails if the root block is not valid
    fn mount<D: BlockDevice + 'static>(block_device: D) -> Result<Self, FsError>;

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

pub trait AsU8Slice: Sized {
    fn as_u8_slice(&mut self) -> &[u8];
    fn as_u8_slice_mut(&mut self) -> &mut [u8];
}

impl<T: Sized> AsU8Slice for T {
    fn as_u8_slice(&mut self) -> &[u8] {
        unsafe {
            &*core::ptr::slice_from_raw_parts_mut((self as *mut T) as *mut u8,
                core::mem::size_of::<T>())
        }
    }
    fn as_u8_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            &mut *core::ptr::slice_from_raw_parts_mut((self as *mut T) as *mut u8,
                core::mem::size_of::<T>())
        }
    }
}
