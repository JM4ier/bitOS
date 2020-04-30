#![no_std]
#![feature(custom_test_frameworks)]

extern crate alloc;

use alloc::{string::{String, ToString}, vec, vec::Vec, boxed::Box};
use core::ops::{Deref, DerefMut};

mod copy;
pub use copy::*;

pub mod ffat;

pub const SEPARATOR: u8 = b'/';

/// Error type for file system errors
#[derive(Debug)]
pub enum FsError {
    /// Some kind of error with the block device
    BlockDeviceError,

    /// file or directory does not exist
    NotFound,

    /// no permission to access path
    AccessViolation,

    /// Superblock invalid or not found
    InvalidSuperBlock,

    /// Invalid internal address to a file, block, inode, etc
    InvalidAddress,

    /// Indicates that there is not enough space on the block device
    NotEnoughSpace,

    /// Some kind of error that is caused by bad programming of the file system
    InternalError(String),

    /// Something that shouldn't be done
    IllegalOperation(String),
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

    /// returns true when the device is read only
    fn is_read_only(&self) -> bool;
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

impl<'b, I: Into<u64> + Sized, T: Sized, B: 'b + BlockDevice> SerdeBlockDevice<I, T> for B {
    fn read(&mut self, index: I, obj: &mut T) -> FsResult<()> {
        self.read_block(index.into(), obj.as_u8_slice_mut())
    }

    fn write(&mut self, index: I, obj: &T) -> FsResult<()> {
        self.write_block(index.into(), obj.as_u8_slice())
    }
}

impl<'b, I: Into<u64> + Sized, T: Sized> SerdeBlockDevice<I, T> for Box<dyn 'b + BlockDevice> {
    fn read(&mut self, index: I, obj: &mut T) -> FsResult<()> {
        self.read_block(index.into(), obj.as_u8_slice_mut())
    }

    fn write(&mut self, index: I, obj: &T) -> FsResult<()> {
        self.write_block(index.into(), obj.as_u8_slice())
    }
}

pub trait StructBlockDevice<D: BlockDevice, B: Deref<Target=D> + DerefMut<Target=D>> {
    fn get<I: Into<u64> + Sized, T: Sized + Default>(&mut self, index: I) -> FsResult<T>;
    fn get_mut<I: Into<u64> + Sized + Copy, T: Sized + Default>(&mut self, index: I) -> FsResult<EditableBlock<'_, T, D, I, B>>;
}

pub struct EditableBlock<'d, T: Sized, D: BlockDevice, I: Into<u64> + Sized, B: Deref<Target=D> + DerefMut<Target=D>> {
    idx: I,
    data: T,
    device: &'d mut B,
}

impl<'d, T: Sized, D: BlockDevice, I: Into<u64> + Sized, B: Deref<Target=D> + DerefMut<Target=D>> EditableBlock<'d, T, D, I, B> {
    pub fn write(self) -> FsResult<()> {
        self.device.write(self.idx, &self.data)
    }
}

impl<'d, T: Sized, D: BlockDevice, I: Into<u64> + Sized, B: Deref<Target=D> + DerefMut<Target=D>> Deref for EditableBlock<'d, T, D, I, B> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'d, T: Sized, D: BlockDevice, I: Into<u64> + Sized, B: Deref<Target=D> + DerefMut<Target=D>> DerefMut for EditableBlock<'d, T, D, I, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<D: BlockDevice, B: Deref<Target=D> + DerefMut<Target=D>> StructBlockDevice<D, B> for B {
    fn get<I: Into<u64> + Sized, T: Sized + Default>(&mut self, idx: I) -> FsResult<T> {
        let mut data = T::default();
        self.read(idx, &mut data)?;
        Ok(data)
    }
    fn get_mut<I: Into<u64> + Sized + Copy, T: Sized + Default>(&mut self, idx: I) -> FsResult<EditableBlock<'_, T, D, I, B>> {
        let mut data = T::default();
        self.read(idx, &mut data)?;
        Ok( EditableBlock {
                idx,
                data,
                device: self,
        })
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

    fn is_read_only(&self) -> bool {
        false
    }
}

pub struct RomDisk<'a> {
    disk: &'a [u8],
}

impl<'a> RomDisk<'a> {
    pub fn new (disk: &'a [u8]) -> Self {
        Self {
            disk,
        }
    }
}

impl<'a> BlockDevice for RomDisk<'a> {
    fn blocksize(&self) -> u64 {
        RAM_BS as u64
    }

    fn blocks(&self) -> u64 {
        self.disk.len() as u64 / RAM_BS as u64
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn read_block(&mut self, index: u64, buffer: &mut [u8]) -> FsResult<()> {
        self.check_args(index, buffer)?;
        let block = &self.disk[(RAM_BS * index as usize)..(RAM_BS * (index+1) as usize)];
        copy(block, buffer, RAM_BS);
        Ok(())
    }

    fn write_block(&mut self, _: u64, _: &[u8]) -> FsResult<()> {
        Err(FsError::IllegalOperation(String::from("Can't write to ROM")))
    }
}

pub struct OwnedDisk {
    data: Vec<[u8; RAM_BS]>,
}

impl OwnedDisk {
    pub fn new(mut data: Vec<[u8; RAM_BS]>) -> Self {
        Self{data}
    }
}

impl BlockDevice for OwnedDisk {
    fn blocksize(&self) -> u64 {
        RAM_BS as u64
    }

    fn blocks(&self) -> u64 {
        self.data.len() as u64
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn read_block(&mut self, index: u64, buffer: &mut [u8]) -> FsResult<()> {
        RamDisk::new(&mut self.data).read_block(index, buffer)
    }

    fn write_block(&mut self, index: u64, buffer: &[u8]) -> FsResult<()> {
        RamDisk::new(&mut self.data).write_block(index, buffer)
    }
}

/// simple struct that stores a file path without the separators
#[derive(Clone)]
pub struct Path {
    path: Vec<Vec<u8>>,
}

/// entry of a directory, a file or directory name
pub type Filename = Vec<u8>;

impl Path {
    pub fn is_root(&self) -> bool {
        self.path.len() == 0
    }
    pub fn parent_dir(&self) -> Option<Path> {
        if self.is_root() {
            None
        } else {
            Some(Self{ path: self.path[..self.path.len()-1].to_vec() })
        }
    }
    pub fn name(&self) -> Option<Vec<u8>> {
        if self.is_root() {
            None
        } else {
            Some(self.path[self.path.len() - 1][..].to_vec())
        }
    }
    pub fn from_str(string: &str) -> Option<Path> {
        let string = string.as_bytes();
        let mut path: Vec<Vec<u8>> = Vec::new();
        let mut token = Vec::new();
        for &ch in string {
            if ch == SEPARATOR {
                if !token.is_empty() {
                    path.push(token);
                    token = Vec::new();
                }
            } else {
                token.push(ch);
            }
        }
        if !token.is_empty() {
            path.push(token);
        }
        Some(Self{path})
    }
    pub fn head_tail(mut self) -> (Option<Filename>, Self) {
        if self.is_root() {
            (None, self)
        } else {
            let tail = self.path.split_off(1);
            let head = self.path[0].clone();
            (Some(head), Self{path: tail})
        }
    }
    pub fn concat(&self, child: Filename) -> Path {
        let mut child_path = self.path.clone();
        child_path.push(child);
        Path {
            path: child_path,
        }
    }
}

pub trait FileSystem<'b> : Sized  {
    /// a handle to a file opened read-only to store read progress
    type ReadProgress;

    /// a handle to a file that is being written to to store write progress
    type WriteProgress;

    /// allowed characters in file names
    fn allowed_chars() -> &'static [u8];

    /// creates a new file system using the `block_device` or fails if the root block is not valid
    fn mount<B: 'b + BlockDevice>(block_device: B) -> FsResult<Self>;
    
    /// Formats the given device and returns the fs
    fn format<B: 'b + BlockDevice>(block_device: B) -> FsResult<Self>;

    /// Returns `true` if the file system if read-only
    fn is_read_only(&self) -> bool;

    /// opens a file and returns write handle
    fn open_write(&mut self, path: Path) -> FsResult<Self::WriteProgress>;

    /// opens a file and returns read handle
    fn open_read(&mut self, path: Path) -> FsResult<Self::ReadProgress>;

    /// reads a directories content
    fn read_dir(&mut self, path: Path) -> FsResult<Vec<Filename>>;

    /// writes to an opened file and updates write progress
    fn write(&mut self, progress: &mut Self::WriteProgress, buffer: &[u8]) -> FsResult<()>;

    /// reads from the opened file to the buffer, returns the number of read bytes
    fn read(&mut self, progress: &mut Self::ReadProgress, buffer: &mut [u8]) -> FsResult<u64>;

    /// performs a seek on the read progress, when being read the next time, it will continue after
    /// the specified `seeking` bytes
    fn seek(&mut self, progress: &mut Self::ReadProgress, seeking: u64) -> FsResult<()>;

    /// deletes a file or directory
    fn delete(&mut self, path: Path) -> FsResult<()>;

    /// clears the file, but does not delete it
    fn clear(&mut self, path: Path) -> FsResult<()>;

    /// creates a new file at the path
    fn create_file(&mut self, path: Path) -> FsResult<()>;

    /// creates a new directory at the path
    fn create_dir(&mut self, path: Path) -> FsResult<()>;

    /// returns `true` when the directory exists, `false` if it doesn't
    fn exists_dir(&mut self, path: Path) -> FsResult<bool>;

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

