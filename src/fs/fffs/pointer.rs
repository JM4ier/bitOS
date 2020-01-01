use super::*;

#[repr(C, align(4096))]
pub struct PointerData {
    pub pointers: [BlockAddr; BLOCK_SIZE / 8],
}

