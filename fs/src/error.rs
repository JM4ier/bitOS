#![no_std]
#![feature(const_generics)]
#![feature(custom_test_frameworks)]

extern crate alloc;
use alloc::string::String;

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

