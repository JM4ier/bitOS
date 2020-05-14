///! Syscall names matched to constants

/// maps a page of virtual memory
pub const VMAP: u64 = 0x1;

/// print to kernel console
pub const KPRINT: u64 = 0x10;

/// open file or directory
pub const OPEN: u64 = 0x20;

/// create file
pub const CREATE: u64 = 0x21;

/// read file
pub const READ: u64 = 0x22;

/// write file
pub const WRITE: u64 = 0x23;

/// remove file
pub const REMOVE: u64 = 0x24;

/// read directory content
pub const READDIR: u64 = 0x25;

/// create directory
pub const MKDIR: u64 = 0x25;

/// remove directory
pub const RMDIR: u64 = 0x26;

/// close file
pub const CLOSE: u64 = 0x27;

