use alloc::string::*;
use alloc::boxed::*;
use alloc::vec::*;
use alloc::collections::BTreeMap;
use spin::RwLock;
use dep::fs::*;

use fs::error::*;
use fs::filesystem::*;
use fs::path::*;

pub type VirtualFile = Box<RwLock<dyn Fn() -> Vec<u8> + Send>>;

enum Entry {
    Directory(VirtualFileSystem),
    File(VirtualFile),
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        if let Self::Directory(_) = self {
            true
        } else {
            false 
        }
    }

    pub fn is_file(&self) -> bool {
        if let Self::File(_) = self {
            true
        } else {
            false 
        }
    }
}

pub struct VirtualFileSystem {
    entries: RwLock<BTreeMap<Filename, Entry>>,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn register_file<F>(&self, path: Path, file: F) -> Result<(), ()> 
        where F: 'static + Fn() -> Vec<u8> + Send
    {
        let (head, tail) = path.head_tail();
        let head = match head {
            Some(head) => head,
            None => return Err(()),
        };

        if tail.is_root() {
            // file
            if self.entries.read().contains_key(&head) {
                return Err(());
            }
            let file = Entry::File(Box::new(RwLock::new(file)));
            self.entries.write().insert(head, file);
            Ok(())
        } else {
            // directory
            if let Some(dir) = self.entries.read().get(&head) {
                if let Entry::Directory(dir) = dir {
                    dir.register_file(tail, file)
                } else {
                    Err(())
                }
            } else {
                let dir = Self::new();
                dir.register_file(tail, file)?;
                self.entries.write().insert(head, Entry::Directory(dir));
                Ok(())
            }
        }
    }

    fn find_entry<R, F: Fn(&Entry) -> R>(&self, path: Path, default: R, fun: F) -> R {
        let (head, tail) = path.head_tail();
        let head = match head {
            Some(h) => h,
            None => return default,
        };

        let entries = self.entries.read();

        let entry = match entries.get(&head) {
            Some(e) => e,
            None => return default,
        };

        if tail.is_root() {
            fun(&entry)
        } else {
            if let Entry::Directory(vfs) = entry {
                vfs.find_entry(tail, default, fun)
            } else {
                default
            }
        }
    }

    fn find_entry_mut<R, F: Fn(&mut Entry) -> R>(fun: F, path: Path) -> Result<R, ()> {
        todo!();
    }

}

impl BaseFileSystem for VirtualFileSystem {
    fn read_dir(&self, path: Path) -> FsResult<Vec<Filename>> {
        self.find_entry(path, Err(FsError::NotFound), |e| {
            if let Entry::Directory(dir) = e {
                Ok(dir.entries.read().iter().map(|(k, v)| k.clone()).collect())
            } else {
                Err(FsError::NotFound)
            }
        })
    }
    fn exists_dir(&self, path: Path) -> FsResult<bool> {
        Ok(self.find_entry(path, false, |e| e.is_dir()))
    }
    fn exists_file(&self, path: Path) -> FsResult<bool> {
        Ok(self.find_entry(path, false, |e| e.is_file()))
    }
}

impl ReadFileSystem for VirtualFileSystem {
    type ReadProgress = (usize, VirtualFile);

    fn open_read(&self, path: Path) -> FsResult<Self::ReadProgress> {
        todo!();
    }

    fn read(&self, progress: &mut Self::ReadProgress, buffer: &mut [u8]) -> FsResult<usize> {
        todo!();
    }

    fn seek(&self, progress: &mut Self::ReadProgress, seeking: usize) -> FsResult<()> {
        progress.0 += seeking;
        Ok(())
    }
}
