pub fn set_bit(bitmap: &mut [u8], index: usize, value: bool) {
    let byte = index / 8;
    let bit = index % 8;
    let bitmask = 1 << bit;
    if value {
        bitmap[byte] |= bitmask;
    } else {
        bitmap[byte] &= !bitmask;
    }
}

pub fn get_bit(bitmap: &[u8], index: usize) -> bool {
    let byte = index / 8;
    let bit = index % 8;
    let bitmask = 1 << bit;
    bitmap[byte] & bitmask > 0
}

