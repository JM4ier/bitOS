use self::super::*;

#[repr(u8)]
pub enum NodeType {
    Directory,
    File,
    SymLink,
}

#[repr(C)]
#[repr(align(128))]
pub struct Node {
    /// the type of the node
    pub node_type: NodeType,

    /// posix-like permissions (rwxrwxrwx)
    pub permission: Permission,

    /// user id of user owning the file
    pub user: u16,

    /// group id of group owning the file
    pub group: u16,

    /// size of the data associated with the node
    /// (file size if it is a file, size of directory info if it is a directory)
    pub size: u64,

    /// time of creation
    pub created: Time,

    /// last time the node was accessed
    pub last_access: Time,

    /// last time the node was modified
    pub last_modified: Time,

    /// time the node was deleted, if it is deleted
    pub deleted: Time,

    /// levels of indirection to the data
    pub indirection_level: u64,

    /// potentially indirect pointers to data
    pub pointers: [BlockAddr; 9],
}

