/// Error type for file system errors
pub enum FsError {
    InvalidIndex,
    MalformedBuffer,
}

/// Result type for file system operations
pub type FsResult = Result<(), FsError>;

/// Generic device that can be read from or written to on a block by block basis
pub trait BlockDevice<const BLOCK_SIZE: usize> {

    /// Number of blocks on the device
    fn blocks(&self) -> u64;

    /// Basic read operation:
    /// reads block from `self` into `buffer`
    fn read_block(&mut self, index: u64, buffer: &mut [u8; BLOCK_SIZE]) -> FsResult;

    /// Basic write operation:
    /// writes `buffer` to `self`
    fn write_block(&mut self, index: u64, buffer: &[u8; BLOCK_SIZE]) -> FsResult;
}

/// This trait allows `Sized` types to be read and written to and from the block device.
/// It uses unsafe memory access to read the bytes of an instance of `T` or write to the bytes of an instance of `T`.
/// It is useful if the generic type `T` is `repr(C)`.
pub trait SerdeBlockDevice<T: Sized, const BLOCK_SIZE: usize> {
    /// Tries to read from `self` and write the raw bytes to `obj` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn read(&mut self, index: u64, obj: &mut T) -> FsResult;

    /// Tries to read from `obj` and write the raw bytes to `self` and
    /// returns `Err(FsError::MalformedBuffer)` if the memory representation of `T` has not the exact size of `BLOCK_SIZE`
    fn write(&mut self, index: u64, obj: &mut T) -> FsResult;
}

/// Simple trait that checks if the I/O operation is valid based on limited information about the block device
trait BlockDeviceArgumentChecks<const BLOCK_SIZE: usize> {
    fn check_args(&self, index: u64) -> FsResult;
}

impl<T: BlockDevice<BLOCK_SIZE>, const BLOCK_SIZE: usize> BlockDeviceArgumentChecks<BLOCK_SIZE> for T {
    fn check_args(&self, index: u64) -> FsResult {
        if index >= self.blocks() {
            Err(FsError::InvalidIndex)
        } else {
            Ok(())
        }
    }
}

impl<T: Sized, B: BlockDevice<BLOCK_SIZE>, const BLOCK_SIZE: usize> SerdeBlockDevice<T, BLOCK_SIZE> for B {
    fn read(&mut self, index: u64, obj: &mut T) -> FsResult {
        if core::mem::size_of::<T>() != BLOCK_SIZE {
            Err(FsError::MalformedBuffer)
        } else {
            Ok(self.read_block(index, unsafe{
                core::mem::transmute::<&mut T, &mut [u8; BLOCK_SIZE]>(obj)
            })?)
        }
    }

    fn write(&mut self, index: u64, obj: &mut T) -> FsResult {
        if core::mem::size_of::<T>() != BLOCK_SIZE {
            Err(FsError::MalformedBuffer)
        } else {
            Ok(self.write_block(index, unsafe {
                core::mem::transmute::<&mut T, &mut [u8; BLOCK_SIZE]>(obj)
            })?)
        }
    }
}

/// RAM disk block size
const RAM_BS: usize = 4096;

/// Simple disk that stores the data in memory
pub struct RamDisk {
    // TODO indexing blocks
}

impl BlockDevice<RAM_BS> for RamDisk {

    fn blocks(&self) -> u64 {
        1 << 16 // 256 MiB
    }
    fn read_block(&mut self, index: u64, buffer: &mut [u8; RAM_BS]) -> FsResult {
        self.check_args(index)?;
        // TODO read
        Ok(())
    }
    fn write_block(&mut self, index: u64, buffer: &[u8; RAM_BS]) -> FsResult {
        self.check_args(index)?;
        // TODO write
        Ok(())
    }
}

