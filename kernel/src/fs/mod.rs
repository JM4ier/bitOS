use alloc::{vec, vec::Vec, boxed::Box};

pub mod fffs;
mod copy;
pub use copy::*;

/// Error type for file system errors
#[derive(Debug)]
pub enum FsError {
    /// Some kind of error with the block device
    BlockDeviceError,

    /// file or directory does not exist
    FileNotFound,

    /// no permission to access path
    AccessViolation,

    /// Superblock invalid or not found
    InvalidSuperBlock,

    /// Invalid internal address to a file, block, inode, etc
    InvalidAddress,

    /// Indicates that there is not enough space on the block device
    NotEnoughSpace,

    /// Some kind of error that is caused by bad programming of the file system
    InternalError,

    /// Something that shouldn't be done
    IllegalOperation,
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
pub trait SerdeBlockDevice<I, T> {
    /// Tries to read from `self` and write the raw bytes to `obj` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn read(&mut self, index: I, obj: &mut T) -> FsResult<()>;

    /// Tries to read from `obj` and write the raw bytes to `self` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn write(&mut self, index: I, obj: &T) -> FsResult<()>;
}

/// Simple trait that checks if the I/O operation is valid based on limited information about the block device
trait BlockDeviceArgumentChecks {
    fn check_args(&self, index: u64, buffer: &[u8]) -> FsResult<()>;
}

impl<D: BlockDevice> BlockDeviceArgumentChecks for D {
    fn check_args(&self, index: u64, buffer: &[u8]) -> FsResult<()> {
        if buffer.len() as u64 != self.blocksize() {
            panic!("Invalid buffer size");
        } else if index >= self.blocks() {
            panic!("Invalid block id");
        } else {
            Ok(())
        }
    }
}

impl<I: Into<u64> + Sized, T: Sized, B: BlockDevice> SerdeBlockDevice<I, T> for B {
    fn read(&mut self, index: I, obj: &mut T) -> FsResult<()> {
        self.read_block(index.into(), obj.as_u8_slice_mut())
    }

    fn write(&mut self, index: I, obj: &T) -> FsResult<()> {
        self.write_block(index.into(), obj.as_u8_slice())
    }
}

impl<I: Into<u64> + Sized, T: Sized> SerdeBlockDevice<I, T> for Box<dyn BlockDevice> {
    fn read(&mut self, index: I, obj: &mut T) -> FsResult<()> {
        self.read_block(index.into(), obj.as_u8_slice_mut())
    }

    fn write(&mut self, index: I, obj: &T) -> FsResult<()> {
        self.write_block(index.into(), obj.as_u8_slice())
    }
}

/// RAM disk block size
const RAM_BS: usize = 4096;

/// Simple disk that stores the data in memory
pub struct RamDisk<'a> {
    disk: &'a mut [[u8; RAM_BS]],
}

impl<'a> RamDisk<'a> {
    pub fn new (disk: &'a mut [[u8; RAM_BS]]) -> Self {
        Self {
            disk,
        }
    }

    fn block_slice(&mut self, index: u64) -> &mut [u8] {
        &mut self.disk[index as usize]
    }
}


impl<'a> BlockDevice for RamDisk<'a> {

    fn blocksize(&self) -> u64 {
        RAM_BS as u64
    }

    fn blocks(&self) -> u64 {
        self.disk.len() as _
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

/// simple struct that stores a file path without the separators
#[derive(Clone)]
pub struct Path {
    path: Vec<Vec<u8>>,
}

impl Path {
    pub fn parent_dir(&self) -> Option<Path> {
        if self.path.len() == 0 {
            None
        } else {
            Some(Self{ path: self.path[..self.path.len()-1].to_vec() })
        }
    }
    pub fn name(&self) -> Option<Vec<u8>> {
        if self.path.len() == 0 {
            None
        } else {
            Some(self.path[self.path.len() - 1][..].to_vec())
        }
    }
    pub fn from_str<B: BlockDevice, T: FileSystem<B>>(string: &str) -> Option<Path> {
        let string = string.as_bytes();
        let mut path: Vec<Vec<u8>> = Vec::new();
        let mut token = Vec::new();
        for &ch in string {
            if ch == T::separator() {
                if !token.is_empty() {
                    path.push(token);
                    token = Vec::new();
                }
            } else if T::allowed_chars().contains(&ch) {
                token.push(ch);
            } else {
                return None;
            }
        }
        Some(Self{path})
    }
}

pub trait FileSystem<B: BlockDevice> : Sized  {
    /// allowed characters in file names
    fn allowed_chars() -> &'static [u8];

    /// separates directory names
    fn separator() -> u8;

    /// creates a new file system using the `block_device` or fails if the root block is not valid
    fn mount(block_device: B) -> Result<Self, FsError>;

    /// opens a file / directory and returns a file descriptor
    fn open(&mut self, path: Path) -> Result<i64, FsError>;

    /// deletes a file or directory
    fn delete(&mut self, path: Path) -> FsResult<()>;

    /// clears the file, but does not delete it
    fn clear(&mut self, path: Path) -> FsResult<()>;

    /// creates a new file at the path
    fn create_file(&mut self, path: Path) -> FsResult<()>;

    /// creates a new directory at the path
    fn create_directory(&mut self, path: Path) -> FsResult<()>;

    /// returns `true` when the directory exists, `false` if it doesn't
    fn exists_directory(&mut self, path: Path) -> FsResult<bool>;

    /// returns `true` when the directory exists, `false` if it doesn't
    fn exists_file(&mut self, path: Path) -> FsResult<bool>;
}

/// Blanket trait that is implemented for every `Sized` type.
/// It allows for an easy conversion from a fixed size type to a slice of `u8`.
pub trait AsU8Slice: Sized {
    /// struct to immutable `u8` slice
    fn as_u8_slice(&self) -> &[u8];
    /// struct to mutable `u8` slice
    fn as_u8_slice_mut(&mut self) -> &mut [u8];
}

impl<T: Sized> AsU8Slice for T {
    fn as_u8_slice(&self) -> &[u8] {
        unsafe {
            &*core::ptr::slice_from_raw_parts((self as *const T) as *const u8,
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

pub fn test_fs() {
    use crate::{serial_println};

    let mut disk = vec![[0u8; RAM_BS]; 8192 + 64];
    let device = RamDisk::new(&mut disk);
    let _fs = fffs::FileSystem::format(device, &[]).unwrap();

    serial_println!("[ok]");
}

#[test_case]
fn test_ramdisk() {
    use crate::{serial_print, serial_println};
    serial_print!("ramdisk test...");
    let mut disk = vec![[0u8; RAM_BS]; 1000];
    let mut device = RamDisk::new(&mut disk);
    let mut block = [1u8; RAM_BS];
    block[89] = 99;
    device.write_block(27, &block).unwrap();
    block[20] = 121;
    device.write_block(100, &block).unwrap();
    block[89] = 2;
    device.read_block(27, &mut block).unwrap();
    assert_eq!(99, block[89]);
    assert_eq!(1, block[3]);
    block[20] = 0;
    device.read_block(100, &mut block).unwrap();
    assert_eq!(121, block[20]);
    assert_eq!(1, block[2888]);
    serial_println!("[ok]");
}
