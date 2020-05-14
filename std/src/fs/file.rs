extern crate alloc;

use core::marker::PhantomData;
use core::convert::*;
use alloc::string::String;

use dep::syscall;
use dep::fs::*;

use crate::syscall::*;
use crate::fs::structs::*;

/// Struct that represents a file handle.
pub struct File<T: Access> {
    /// File Descriptor
    fd: i64,
    is_open: bool,
    _phantom: PhantomData<T>,
}

fn fd_to_file<T: Access>(fd: i64) -> FsResult<File<T>> {
    let error = FsError::try_from(fd);
    if let Ok(err) = error {
        Err(err)
    } else {
        Ok(File::<T> {
            fd,
            _phantom: PhantomData,
            is_open: true,
        })
    }
}

impl<T: Access> File<T> {
    /// Open the file at the location given by the path.
    /// If opening the file succeeded (process has permissions and file exists),
    /// this returns a `File` struct.
    pub fn open(path: &Path) -> FsResult<File<T>> {
        let path = path.to_string();
        let path = path.as_bytes();
        let fd = unsafe {
            syscall!(syscall::OPEN, path.as_ptr(), path.len(), T::flags())
        };
        fd_to_file::<T>(fd)
    }

    /// closes the file
    pub fn close(&mut self) {
        if self.is_open {
            unsafe {
                syscall!(syscall::CLOSE, self.fd);
            }
            self.is_open = false;
        }
    }
}

impl<T: Access> Drop for File<T> {
    fn drop(&mut self) {
        self.close();
    }
}

impl File<Read> {
    /// Reads file content into the provided buffer.
    /// Returns the number of bytes that were read.
    pub fn read(&mut self, bytes: &mut [u8]) -> FsResult<usize> {
        if !self.is_open {
            return Err(FsError::IllegalOperation);
        }

        let bytes_read = unsafe {
            syscall!(syscall::READ, self.fd, bytes.as_ptr(), bytes.len())
        };
        if bytes_read < 0 {
            // read after close or similar stuff
            Err(FsError::IllegalOperation)
        } else {
            Ok(bytes_read as _)
        }
    }
}

impl File<Write> {
    /// Creates a new file at the specified location.
    /// If this process is allowed to create that file,
    /// this function returns a `File` handle
    pub fn create(location: String) -> FsResult<File<Write>> {
        let path = location.as_bytes();
        let fd = unsafe {
            syscall!(syscall::CREATE, path.as_ptr(), path.len(), Write::flags())
        };
        fd_to_file::<Write>(fd)
    }

    /// Writes the buffer passed as argument to the end of the file
    pub fn write(&mut self, bytes: &[u8]) -> FsResult<()> {
        if !self.is_open {
            return Err(FsError::IllegalOperation);
        }
        let status_code = unsafe {
            syscall!(syscall::WRITE, self.fd, bytes.as_ptr(), bytes.len())
        };
        if status_code != 0 {
            Err(FsError::try_from(status_code).unwrap_or(FsError::IllegalOperation))
        } else {
            Ok(())
        }
    }
}

