use alloc::{vec, vec::Vec};
use alloc::collections::BTreeMap;
use alloc::string::*;
use alloc::boxed::Box;
use spin::{Once, Mutex};
use crate::{print, fs::{*, ffat::*}};
use core::ops::DerefMut;
use core::marker::*;

use fs::error::*;
use fs::filesystem::*;
use fs::block::*;
use fs::memory_devices::*;
use fs::path::Path;


static DISK_IMAGE: &'static [u8] = include_bytes!("../../../disk.img");

static mut FS: Once<Mutex<RootFileSystem>> = Once::new();

/// initializes the file system if it isn't already
pub fn init() {
    // do something with the DISK so that it gets initialized
    fs();
}

/// returns an exclusive handle to the file system
/// which is lazily initialized
pub fn fs() -> impl DerefMut<Target = RootFileSystem> {
    // copy the entire read-only DISK_IMAGE to be able to write to it
    // does not persist accross reboots
    unsafe {
        FS.call_once(|| { 
            let mut img = vec![0u8; DISK_IMAGE.len()];
            for i in 0..img.len() {
                img[i] = DISK_IMAGE[i];
            }
            use fs::filesystem::*;
            let ffat = {
                if let Ok(ffat) = FFAT::mount(OwnedDisk{ data: img }) {
                    ffat
                } else {
                    panic!("could not mount disk")
                }
            };
            let mut rfs = RootFileSystem::new();
            if let Err(fs) = rfs.mount(ffat, Path::root()) {
                print!("failed to mount ffat\n");
            }
            Mutex::new(rfs)
        }).lock()
    }
}

/// reads the entire file specified by the path and returns it in a vec
pub fn read_all(path: Path) -> FsResult<Vec<u8>> {
    let mut fs = fs();
    let mut handle = fs.open_read(path)?;

    let mut vec = Vec::new();
    let mut buffer = [0u8; 4096];

    loop {
        let bytes_read = fs.read(handle, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        vec.append(&mut buffer[..bytes_read as usize].to_vec());
    }
    Ok(vec)
}

pub struct RootFileSystem {
    /// mounted file systems
    file_systems: Vec<Option<Box<dyn 'static + Mounted>>>,

    /// next unique file descriptor
    next_fd: i64,

    /// open 'read' file descriptors mapped to index of file system
    files_read: BTreeMap<i64, usize>,

    /// open 'write' file descriptors mapped to index of file system
    files_write: BTreeMap<i64, usize>,
}

pub fn map_err(result: FsResult<i64>) -> i64 {
    match result {
        Ok(fd) => fd,
        Err(err) => error_to_const(err),
    }
}

use dep::fs::error::*;
pub fn error_to_const(err: FsError) -> i64 {
    match err {
        FsError::NotFound => NOT_FOUND,
        FsError::AccessViolation => ACCESS_VIOLATION,
        FsError::IllegalOperation(_) => ILLEGAL,
        _ => OTHER,
    }
}

impl RootFileSystem {
    pub fn new() -> Self {
        Self {
            file_systems: Vec::new(),
            next_fd: 1,
            files_read: BTreeMap::new(),
            files_write: BTreeMap::new(),
        }
    }

    /// mounts a file system at a given path or fails if
    /// the mount directory has conflicting entries
    pub fn mount<T, B, const BS: usize> (&mut self, mut fs: T, mount_point: Path) -> Result<(), T>
    where T: 'static + CompleteFileSystem<B, BS>,
          B: 'static + RWBlockDevice<BS>
    {
        if mount_point.is_root() && self.mount_count() == 0 {
            self.file_systems.push(Some(Box::new(MountedFileSystem::new(fs, mount_point))));
            Ok(())
        } else {
            if let Ok(fs_root_entries) = fs.read_dir(Path::root()) {
                match self.read_dir(mount_point.clone()) {
                    Ok(mounted_root_entries) => {
                        for mounted_entry in mounted_root_entries {
                            if fs_root_entries.contains(&mounted_entry) {
                                return Err(fs);
                            }
                        }
                        self.file_systems.push(Some(Box::new(MountedFileSystem::new(fs, mount_point))));
                        Ok(())
                    },
                    _ => Err(fs),
                }
            } else {
                Err(fs)
            }
        }
    }

    pub fn mount_count(&self) -> usize {
        self.file_systems
            .iter()
            .filter(|fs| fs.is_some())
            .count()
    }

    fn get_free_fd(&mut self) -> i64 {
        let fd = self.next_fd;
        self.next_fd += 1;
        fd
    }

    pub fn open_write(&mut self, path: Path) -> FsResult<i64> {
        let fd = self.get_free_fd();
        let (fs, path) = self.suitable_fs(path)?;
        self.file_systems[fs].as_mut().unwrap().open_write(fd, path)?;
        self.files_write.insert(fd, fs);
        Ok(fd)
    }

    pub fn open_read(&mut self, path: Path) -> FsResult<i64> {
        let fd = self.get_free_fd();
        let (fs, path) = self.suitable_fs(path)?;
        self.file_systems[fs].as_mut().unwrap().open_read(fd, path)?;
        self.files_read.insert(fd, fs);
        Ok(fd)
    }

    pub fn read_dir(&mut self, path: Path) -> FsResult<Vec<Filename>> {
        self.apply_to_suitable_fs(path, |fs, path| fs.read_dir(path))
    }

    pub fn write(&mut self, fd: i64, buffer: &[u8]) -> FsResult<()> {
        if let Some(&fs) = self.files_write.get(&fd) {
            if let Some(fs) = self.file_systems[fs].as_mut() {
                fs.write(fd, buffer)
            } else {
                Err(FsError::IllegalOperation("write after unmount".to_string()))
            }
        } else {
            Err(FsError::IllegalOperation("no such file descriptor".to_string()))
        }
    }

    pub fn read(&mut self, fd: i64, buffer: &mut [u8]) -> FsResult<usize> {
        if let Some(&fs) = self.files_read.get(&fd) {
            if let Some(fs) = self.file_systems[fs].as_mut() {
                fs.read(fd, buffer)
            } else {
                Err(FsError::IllegalOperation("read after unmount".to_string()))
            }
        } else {
            Err(FsError::IllegalOperation("no such file descriptor".to_string()))
        }
    }

    pub fn seek(&mut self, fd: i64, seek: usize) -> FsResult<()> {
        if let Some(&fs) = self.files_read.get(&fd) {
            if let Some(fs) = self.file_systems[fs].as_mut() {
                fs.seek(fd, seek)
            } else {
                Err(FsError::IllegalOperation("seek after unmount".to_string()))
            }
        } else {
            Err(FsError::IllegalOperation("no such file descriptor".to_string()))
        }
    }

    pub fn delete(&mut self, path: Path) -> FsResult<()> {
        self.apply_to_suitable_fs(path, |fs, path| fs.delete(path))
    }

    pub fn clear(&mut self, path: Path) -> FsResult<()> {
        self.apply_to_suitable_fs(path, |fs, path| fs.clear(path))
    }

    pub fn create_file(&mut self, path: Path) -> FsResult<()> {
        self.apply_to_suitable_fs(path, |fs, path| fs.create_file(path))
    }

    pub fn create_dir(&mut self, path: Path) -> FsResult<()> {
        self.apply_to_suitable_fs(path, |fs, path| fs.create_dir(path))
    }

    pub fn exists_dir(&mut self, path: Path) -> FsResult<bool> {
        self.apply_to_suitable_fs(path, |fs, path| fs.exists_dir(path))
    }

    pub fn exists_file(&mut self, path: Path) -> FsResult<bool> {
        self.apply_to_suitable_fs(path, |fs, path| fs.exists_file(path))
    }

    /// finds the suitable mounted file system and applies a function at the given path
    fn apply_to_suitable_fs<R, F> (&mut self, path: Path, fun: F) -> FsResult<R> 
        where F: Fn(&mut Box<dyn Mounted>, Path) -> FsResult<R>
    {
        let (suitable, path) = self.suitable_fs(path)?;
        let mut fs = self.file_systems[suitable].as_mut().unwrap();
        fun(&mut fs, path)
    }

    /// returns the suitable mounted file system for the given path
    fn suitable_fs(&mut self, path: Path) -> FsResult<(usize, Path)> {
        let mut shortest_dist = usize::MAX;
        let mut suitable_fs = None;

        for (i, fs) in self.file_systems.iter_mut().enumerate() {
            if let Some(fs) = fs {
                if let Some(rel) = path.clone().relative_to(fs.mount_point().clone()) {
                    if rel.len() < shortest_dist {
                        shortest_dist = rel.len();
                        suitable_fs = Some((i, rel));
                    }
                }
            }
        }

        suitable_fs.ok_or(FsError::IllegalOperation("Path is not contained in a mounted file system".to_string()))
    }
}

trait Mounted {
    fn mount_point(&mut self) -> &mut Path;
    fn open_read(&mut self, fd: i64, path: Path) -> FsResult<()>;
    fn open_write(&mut self, fd: i64, path: Path) -> FsResult<()>;
    fn read(&mut self, fd: i64, buffer: &mut [u8]) -> FsResult<usize>;
    fn write(&mut self, fd: i64, buffer: &[u8]) -> FsResult<()>;
    fn seek(&mut self, fd: i64, seek: usize) -> FsResult<()>;
    fn read_dir(&mut self, path: Path) -> FsResult<Vec<Filename>>;
    fn delete(&mut self, path: Path) -> FsResult<()>;
    fn clear(&mut self, path: Path) -> FsResult<()>;
    fn create_file(&mut self, path: Path) -> FsResult<()>;
    fn create_dir(&mut self, path: Path) -> FsResult<()>;
    fn exists_file(&mut self, path: Path) -> FsResult<bool>;
    fn exists_dir(&mut self, path: Path) -> FsResult<bool>;
}

struct MountedFileSystem<T, B, const BS: usize>
where T: CompleteFileSystem<B, BS>, B: RWBlockDevice<BS> {
    fs: T,
    mount_point: Path,
    files_read: BTreeMap<i64, T::ReadProgress>,
    files_write: BTreeMap<i64, T::WriteProgress>,
    _phantom: PhantomData<B>,
}

impl<T, B, const BS: usize> MountedFileSystem<T, B, BS>
where T: CompleteFileSystem<B, BS>, B: RWBlockDevice<BS> {
    fn new(fs: T, mount_point: Path) -> Self {
        Self {
            fs,
            mount_point,
            files_read: BTreeMap::new(),
            files_write: BTreeMap::new(),
            _phantom: PhantomData,
        }
    }
}

fn no_such_fd<T>() -> FsResult<T> {
    Err(FsError::IllegalOperation("This file descriptor does not exist".to_string()))
}

impl<T, B, const BS: usize> Mounted for MountedFileSystem<T, B, BS>
where T: CompleteFileSystem<B, BS>, B: RWBlockDevice<BS> {
    fn mount_point(&mut self) -> &mut Path {
        &mut self.mount_point
    }

    fn open_read(&mut self, fd: i64, path: Path) -> FsResult<()> {
        let rp = self.fs.open_read(path)?;
        self.files_read.insert(fd, rp);
        Ok(())
    }

    fn open_write(&mut self, fd: i64, path: Path) -> FsResult<()> {
        let wp = self.fs.open_write(path)?;
        self.files_write.insert(fd, wp);
        Ok(())
    }

    fn read(&mut self, fd: i64, buffer: &mut [u8]) -> FsResult<usize> {
        match self.files_read.get_mut(&fd) {
            None => no_such_fd(),
            Some(mut rp) => {
                Ok(self.fs.read(&mut rp, buffer)? as usize)
            }
        }
    }

    fn write(&mut self, fd: i64, buffer: &[u8]) -> FsResult<()> {
        match self.files_write.get_mut(&fd) {
            None => no_such_fd(),
            Some(mut wp) => self.fs.write(&mut wp, buffer),
        }
    }

    fn seek(&mut self, fd: i64, seek: usize) -> FsResult<()> {
        match self.files_read.get_mut(&fd) {
            None => no_such_fd(),
            Some(mut rp) => self.fs.seek(&mut rp, seek as _),
        }
    }

    fn read_dir(&mut self, path: Path) -> FsResult<Vec<Filename>> {
        self.fs.read_dir(path)
    }

    fn delete(&mut self, path: Path) -> FsResult<()> {
        self.fs.delete(path)
    }

    fn clear(&mut self, path: Path) -> FsResult<()> {
        self.fs.clear(path)
    }

    fn create_file(&mut self, path: Path) -> FsResult<()> {
        self.fs.create_file(path)
    }

    fn create_dir(&mut self, path: Path) -> FsResult<()> {
        self.fs.create_dir(path)
    }

    fn exists_dir(&mut self, path: Path) -> FsResult<bool> {
        self.fs.exists_dir(path)
    }

    fn exists_file(&mut self, path: Path) -> FsResult<bool> {
        self.fs.exists_file(path)
    }
}

