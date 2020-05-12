#![no_std]
#![feature(const_generics)]
#![feature(custom_test_frameworks)]

pub mod error;
pub mod copy;
pub mod block;
pub mod memory_devices;
pub mod filesystem;

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

