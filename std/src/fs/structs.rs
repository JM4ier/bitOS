use core::convert::TryFrom;
use dep::fs::error::*;

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

#[derive(Debug)]
#[repr(i64)]
pub enum FsError {
    /// file or directory not found
    NotFound,
    /// no access rights
    AccessViolation,
    /// an illegal operation, e.g. reading a file after closing it
    IllegalOperation,
}

use FsError::*;
impl TryFrom<i64> for FsError {
    type Error = ();

    fn try_from(val: i64) -> Result<Self, Self::Error> {
        match val {
            NOT_FOUND => Ok(NotFound),
            ACCESS_VIOLATION => Ok(AccessViolation),
            ILLEGAL => Ok(IllegalOperation),
            _ => Err(()),
        }
    }
}

pub type FsResult<T> = Result<T, FsError>;

