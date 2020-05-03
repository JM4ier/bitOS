use alloc::vec::Vec;
use alloc::string::String;

pub mod error;

pub const SEPARATOR: u8 = b'/';

pub type Filename = Vec<u8>;

/// Represents a file path
#[derive(Clone, PartialEq, Eq)]
pub struct Path {
    path: Vec<Filename>,
}


impl Path {

    /// root of the file system
    pub fn root() -> Self {
        Self {
            path: Vec::with_capacity(0),
        }
    }

    /// `true` when the path is the root of the file system, i.e. `SEPARATOR`
    pub fn is_root(&self) -> bool {
        self.path.len() == 0
    }

    /// immediate parent directory if the path is not root
    pub fn parent_dir(&self) -> Option<Self> {
        if self.is_root() {
            None
        } else {
            Some(Self{ path: self.path[..self.path.len()-1].to_vec() })
        }
    }

    /// name of the file or directory the path describes
    /// `None` if it is root
    /// `Some(name)` otherwise
    pub fn name(&self) -> Option<Filename> {
        if self.is_root() {
            None
        } else {
            Some(self.path[self.path.len() - 1][..].to_vec())
        }
    }

    pub fn new<S>(path: S) -> Option<Self>
        where S: AsRef<str>
    {
        Self::from_str(path.as_ref())
    }

    pub fn from_str(string: &str) -> Option<Self> {
        let string = string.as_bytes();
        let mut path: Vec<Vec<u8>> = Vec::new();
        let mut token = Vec::new();
        for &ch in string {
            if ch == SEPARATOR {
                if !token.is_empty() {
                    path.push(token);
                    token = Vec::new();
                }
            } else {
                token.push(ch);
            }
        }
        if !token.is_empty() {
            path.push(token);
        }
        Some(Self{path})
    }

    pub fn to_string(&self) -> String {
        let mut path = String::new();

        for part in self.path.iter() {
            path.push(SEPARATOR as char);
            path.push_str(&String::from_utf8(part.to_vec()).unwrap());
        }

        if path.len() == 0 {
            path.push(SEPARATOR as char);
        }

        path
    }

    pub fn head_tail(mut self) -> (Option<Filename>, Self) {
        if self.is_root() {
            (None, self)
        } else {
            let tail = self.path.split_off(1);
            let head = self.path[0].clone();
            (Some(head), Self{path: tail})
        }
    }

    /// concatenates a file or directory name to the path
    pub fn concat(&self, child: Filename) -> Self {
        let mut child_path = self.path.clone();
        child_path.push(child);
        Self {
            path: child_path,
        }
    }

    pub fn relative_to(self, ancestor: Self) -> Option<Self> {
        if let (Some(anc_head), anc_tail) = ancestor.head_tail() {
            if let (Some(head), tail) = self.head_tail() {
                if anc_head == head {
                    tail.relative_to(anc_tail)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            Some(self)
        }
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

}

