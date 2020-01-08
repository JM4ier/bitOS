use core::marker::PhantomData;
extern crate alloc;
use alloc::string::String;

use crate::syscall::*;
use crate::fs::structs::*;

/// Struct that represents a file handle.
pub struct File<T: Access> {
    /// File Descriptor
    fd: i64,
    _phantom: PhantomData<T>,
}

fn fd_to_file<T: Access>(fd: i64) -> FsResult<File<T>> {
    match fd {
        -1 => Err(FsError::NotFound),
        -2 => Err(FsError::AccessViolation),
        fd => Ok(File::<T>{
            fd,
            _phantom: PhantomData
        }),
    }
}

impl<T: Access> File<T> {
    /// Open the file at the location given by the string.
    /// If opening the file succeeded (process has permissions and file exists),
    /// this returns a `File` struct.
    pub fn open(location: String) -> FsResult<File<T>> {
        let path = location.as_bytes();
        let fd = unsafe {
            syscall!(SYS_OPEN, path.as_ptr(), path.len(), T::flags())
        };
        fd_to_file::<T>(fd)
    }

    /// Consumes and closes the file
    pub fn close(self) {
        unsafe {
            syscall!(SYS_CLOSE, self.fd);
        }
    }
}

impl File<Read> {
    /// Reads file content into the provided buffer.
    /// Returns the number of bytes that were read.
    pub fn read(&mut self, bytes: &mut [u8]) -> usize {
        let bytes_read = unsafe {
            syscall!(SYS_READ, self.fd, bytes.as_ptr(), bytes.len())
        };
        bytes_read as _
    }
}

impl File<Write> {
    /// Creates a new file at the specified location.
    /// If this process is allowed to create that file,
    /// this function returns a `File` handle
    pub fn create(location: String) -> FsResult<File<Write>> {
        let path = location.as_bytes();
        let fd = unsafe {
            syscall!(SYS_CREATE, path.as_ptr(), path.len(), Write::flags())
        };
        fd_to_file::<Write>(fd)
    }

    /// Writes the buffer passed as argument to the end of the file
    pub fn write(&mut self, bytes: &[u8]) {
        unsafe {
            syscall!(SYS_WRITE, self.fd, bytes.as_ptr(), bytes.len());
        }
    }
}

