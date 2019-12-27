pub enum FsError {
    InvalidIndex,
    MalformedBuffer,
}

pub type FsResult = Result<(), FsError>;

pub trait BlockDevice {
    const BLOCK_SIZE: usize;
    fn blocks(&self) -> u64;
    fn read(&mut self, index: u64, buffer: &mut [u8]) -> FsResult;
    fn write(&mut self, index: u64, buffer: &[u8]) -> FsResult;
}

trait BlockDeviceArgumentChecks {
    fn check_args(&self, index: u64, buffer: &[u8]) -> FsResult;
}

impl<T: BlockDevice> BlockDeviceArgumentChecks for T {
    fn check_args(&self, index: u64, buffer: &[u8]) -> FsResult {
        if buffer.len() != Self::BLOCK_SIZE {
            Err(FsError::MalformedBuffer)
        } else if index >= self.blocks() {
            Err(FsError::InvalidIndex)
        } else {
            Ok(())
        }
    }
}

pub struct RamDisk {
    // TODO indexing blocks
}

impl BlockDevice for RamDisk {
    const BLOCK_SIZE: usize = 4096;

    fn blocks(&self) -> u64 {
        1 << 16 // 256 MiB
    }
    fn read(&mut self, index: u64, buffer: &mut [u8]) -> FsResult {
        self.check_args(index, buffer)?;
        // TODO read
        Ok(())
    }
    fn write(&mut self, index: u64, buffer: &[u8]) -> FsResult {
        self.check_args(index, buffer)?;
        // TODO write
        Ok(())
    }
}

