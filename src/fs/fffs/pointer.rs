use super::*;

/// Struct that stores potentially indirect pointers to blocks belonging to a node.
/// Depending on the size of the file/directory, there may be several layers of indirection
/// between the node and the data, e.g. node points to pointers, that point to pointers,
/// that point to files
#[repr(C, align(4096))]
pub struct PointerData {
    pub pointers: [BlockAddr; BLOCK_SIZE / 8],
}

impl PointerData {
    /// creates a pointer table where every pointer points to 'null'.
    /// Useful for partial initialization or deserialization of data on disk.
    pub fn empty () -> Self {
        Self {
            pointers: [BlockAddr::null(); BLOCK_SIZE / 8],
        }
    }
}

