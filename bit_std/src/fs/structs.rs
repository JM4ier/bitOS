pub struct Read;
pub struct Write;

pub trait Access {
    fn flags() -> u64;
}

impl Access for Read {
    fn flags() -> u64 {
        0
    }
}

impl Access for Write {
    fn flags() -> u64 {
        1
    }
}

#[repr(C)]
pub enum FsError {
    /// file or directory not found
    NotFound,
    /// no access rights
    AccessViolation,
    /// an illegal operation, e.g. reading a file after closing it
    IllegalOperation,
}

pub type FsResult<T> = Result<T, FsError>;

