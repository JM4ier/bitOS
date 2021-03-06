extern crate alloc;

use alloc::vec::Vec;
use alloc::boxed::Box;
pub use dep::fs::*;

use crate::error::*;
use crate::block::*;

/// Basic File-System functions
pub trait BaseFileSystem {
    /// reads the contents of a directory in the file system
    fn read_dir(&self, path: Path) -> FsResult<Vec<Filename>> { Err(FsError::AccessViolation) }
    /// checks wether a directory exists or not
    fn exists_dir(&self, path: Path) -> FsResult<bool> { Err(FsError::AccessViolation) }
    /// check wether a file exists or not
    fn exists_file(&self, path: Path) -> FsResult<bool> { Err(FsError::AccessViolation) }
}

/// Functions for a file system that can be mounted
pub trait MountedFileSystem<B: ?Sized + RWBlockDevice>
where Self: Sized {
    /// name of the file system
    fn name() -> &'static str;
    /// mounts a BlockDevice 
    fn mount(device: Box<B>) -> Result<Self, Box<B>>;
    /// formats the given BlockDevice with the File System
    fn format(device: Box<B>) -> Result<Self, Box<B>>;
    /// returns the mounted `BlockDevice`
    fn inner(self) -> Box<B>;
}

/// Functions for a file system that supports reading files
pub trait ReadFileSystem {
    /// progress how far a file has been read, used as a handle to repeatedly read from the same
    /// file
    type ReadProgress: Send; 
    /// opens a file and returns a progress handle to that file
    fn open_read(&self, path: Path) -> FsResult<Self::ReadProgress> { Err(FsError::AccessViolation) }
    /// reads from a file using the progress handle
    fn read(&self, progress: &mut Self::ReadProgress, buffer: &mut [u8]) -> FsResult<usize> { Err(FsError::AccessViolation) }
    /// seeks forward in the file
    fn seek(&self, progress: &mut Self::ReadProgress, seek: usize) -> FsResult<()> { Err(FsError::AccessViolation) }
}

/// Functions for a file system that supports writing files
pub trait WriteFileSystem {
    /// progress how far a file has been written, used as a handle to repeatedly write to the same
    /// file
    type WriteProgress: Send;
    /// opens a file and returns a write handle to that file
    fn open_write(&mut self, path: Path) -> FsResult<Self::WriteProgress> { Err(FsError::AccessViolation) }
    /// writes to a file and updates the progress
    fn write(&mut self, progress: &mut Self::WriteProgress, buffer: &[u8]) -> FsResult<()> { Err(FsError::AccessViolation) }
}

/// Functions for a file system that supports managing directories and file creation
pub trait ManageFileSystem {
    /// deletes a file or directory
    fn delete(&mut self, path: Path) -> FsResult<()> { Err(FsError::AccessViolation) }
    /// clears a file or directory
    fn clear(&mut self, path: Path) -> FsResult<()> { Err(FsError::AccessViolation) }
    /// creates a new, empty file
    fn create_file(&mut self,  path: Path) -> FsResult<()> { Err(FsError::AccessViolation) }
    /// creates a new, empty directory
    fn create_dir(&mut self,  path: Path) -> FsResult<()> { Err(FsError::AccessViolation) }
}

pub trait FunctionalFileSystem : BaseFileSystem + ReadFileSystem + WriteFileSystem + ManageFileSystem {}
impl<FS> FunctionalFileSystem for FS where FS: BaseFileSystem + ReadFileSystem + WriteFileSystem + ManageFileSystem {}

pub trait CompleteFileSystem<B> : FunctionalFileSystem + MountedFileSystem<B> where B: ?Sized + RWBlockDevice {}
impl<FS, B> CompleteFileSystem<B> for FS where FS: FunctionalFileSystem + MountedFileSystem<B>, B: ?Sized + RWBlockDevice {}

