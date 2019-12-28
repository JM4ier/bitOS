pub trait BlockSize<const SIZE: usize> {
    fn mem_block () -> [u8; SIZE];
}

pub struct Size4KiB;
impl BlockSize<4096> for Size4KiB {
    fn mem_block() -> [u8; 4096] {
        [0; 4096]
    }
}

