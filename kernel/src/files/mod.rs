use alloc::{vec, vec::Vec};
use alloc::collections::BTreeMap;
use alloc::string::*;
use alloc::boxed::Box;
use spin::*;
use crate::{print, println, fs::{*, ffat::*}};
use core::ops::DerefMut;
use core::marker::*;

use fs::error::*;
use fs::filesystem::*;
use fs::block::*;
use fs::memory_devices::*;
use fs::path::Path;


static DISK_IMAGE: &'static [u8] = include_bytes!("../../../disk.img");

static FS: Once<Mutex<RootFileSystem>> = Once::new();

static FILE_SYSTEMS: Once<Mutex<Vec<MountData>>> = Once::new();


/// initializes the file system if it isn't already
pub fn init() {
    // register all file system types that can mount disks
    // currently only FFAT
    register_fs::<FFAT<_>>();

    // do something with the DISK so that it gets initialized
    fs();

    // copy the entire read-only DISK_IMAGE to be able to write to it
    // does not persist accross reboots
    let mut img = vec![0u8; DISK_IMAGE.len()];
    for i in 0..img.len() {
        img[i] = DISK_IMAGE[i];
    }

    mount_disk(OwnedDisk{ data: img }, Path::root()).expect("could not mount root file system");
}

struct MountData {
    name: String,
    mount: Box<dyn Fn(Box<dyn RWBlockDevice>, Path) -> Result<(), Box<dyn RWBlockDevice>> + Send>,
    format: Box<dyn Fn(Box<dyn RWBlockDevice>, Path) -> Result<(), Box<dyn RWBlockDevice>> + Send>,
}

/// returns an exclusive handle to the file system
pub fn fs() -> impl DerefMut<Target = RootFileSystem> {
    FS.call_once(|| Mutex::new(RootFileSystem::new())).lock()
}

/// returns an exclusive handle to the registered mountable file systems
fn file_systems() -> impl DerefMut<Target = Vec<MountData>> {
    FILE_SYSTEMS.call_once(|| Mutex::new(Vec::new())).lock()
}

pub fn register_fs<FS: 'static + CompleteFileSystem<dyn RWBlockDevice> + Send>() {
    let mount = Box::new(|dev, path| {
        fs().attach(FS::mount(dev)?, path).map_err(|fs| fs.inner())?;
        Ok(())
    });

    let format = Box::new(|dev, path| {
        fs().attach(FS::format(dev)?, path).map_err(|fs| fs.inner())?;
        Ok(())
    });
        
    let data = MountData {
        name: FS::name().to_string(),
        mount,
        format,
    };

    file_systems().push(data);
}

/// mounts a disk partition by trying for each file system if it fits
pub fn mount_disk<D: 'static + RWBlockDevice>(disk: D, path: Path) -> Result<(), ()> {
    let mut disk: Box<dyn RWBlockDevice> = Box::new(disk);
    for fs in file_systems().iter() {
        match (fs.mount)(disk, path.clone()) {
            Ok(_) => return Ok(()),
            Err(d) => disk = d,
        }
    }
    Err(())
}

/// reads the entire file specified by the path and returns it in a vec
pub fn read_all(path: Path) -> FsResult<Vec<u8>> {
    let mut fs = fs();
    let handle = fs.open_read(path)?;

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
    /// attached file systems
    file_systems: Vec<Option<Box<dyn 'static + Attached + Send>>>,

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

    /// attaches a file system at a given path or fails if
    /// the attach directory has conflicting entries
    pub fn attach<T> (&mut self, mut fs: T, attach_point: Path) -> Result<(), T>
    where T: 'static + FunctionalFileSystem + Send
    {
        if attach_point.is_root() && self.attach_count() == 0 {
            self.file_systems.push(Some(Box::new(AttachedFileSystem::new(fs, attach_point))));
            Ok(())
        } else {
            if let Ok(fs_root_entries) = fs.read_dir(Path::root()) {
                match self.read_dir(attach_point.clone()) {
                    Ok(mounted_root_entries) => {
                        for mounted_entry in mounted_root_entries {
                            if fs_root_entries.contains(&mounted_entry) {
                                return Err(fs);
                            }
                        }
                        self.file_systems.push(Some(Box::new(AttachedFileSystem::new(fs, attach_point))));
                        Ok(())
                    },
                    _ => Err(fs),
                }
            } else {
                Err(fs)
            }
        }
    }

    pub fn attach_count(&self) -> usize {
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
        self.apply_to_suitable_fs(path, |fs, path| fs.inner_fs_mut().read_dir(path))
    }

    pub fn write(&mut self, fd: i64, buffer: &[u8]) -> FsResult<()> {
        if let Some(&fs) = self.files_write.get(&fd) {
            if let Some(fs) = self.file_systems[fs].as_mut() {
                fs.write(fd, buffer)
            } else {
                Err(FsError::IllegalOperation("write after detached".to_string()))
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
                Err(FsError::IllegalOperation("read after detached".to_string()))
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
                Err(FsError::IllegalOperation("seek after detached".to_string()))
            }
        } else {
            Err(FsError::IllegalOperation("no such file descriptor".to_string()))
        }
    }

    pub fn delete(&mut self, path: Path) -> FsResult<()> {
        self.apply_to_suitable_fs(path, |fs, path| fs.inner_fs_mut().delete(path))
    }

    pub fn clear(&mut self, path: Path) -> FsResult<()> {
        self.apply_to_suitable_fs(path, |fs, path| fs.inner_fs_mut().clear(path))
    }

    pub fn create_file(&mut self, path: Path) -> FsResult<()> {
        self.apply_to_suitable_fs(path, |fs, path| fs.inner_fs_mut().create_file(path))
    }

    pub fn create_dir(&mut self, path: Path) -> FsResult<()> {
        self.apply_to_suitable_fs(path, |fs, path| fs.inner_fs_mut().create_dir(path))
    }

    pub fn exists_dir(&mut self, path: Path) -> FsResult<bool> {
        self.apply_to_suitable_fs(path, |fs, path| fs.inner_fs_mut().exists_dir(path))
    }

    pub fn exists_file(&mut self, path: Path) -> FsResult<bool> {
        self.apply_to_suitable_fs(path, |fs, path| fs.inner_fs_mut().exists_file(path))
    }

    /// finds the suitable attached file system and applies a function at the given path
    fn apply_to_suitable_fs<R, F> (&mut self, path: Path, fun: F) -> FsResult<R> 
        where F: Fn(&mut Box<dyn Attached + Send>, Path) -> FsResult<R>
    {
        let (suitable, path) = self.suitable_fs(path)?;
        let mut fs = self.file_systems[suitable].as_mut().unwrap();
        fun(fs, path)
    }

    /// returns the suitable attached file system for the given path
    fn suitable_fs(&mut self, path: Path) -> FsResult<(usize, Path)> {
        let mut shortest_dist = usize::MAX;
        let mut suitable_fs = None;

        for (i, fs) in self.file_systems.iter_mut().enumerate() {
            if let Some(fs) = fs {
                if let Some(rel) = path.clone().relative_to(fs.attach_point().clone()) {
                    if rel.len() < shortest_dist {
                        shortest_dist = rel.len();
                        suitable_fs = Some((i, rel));
                    }
                }
            }
        }

        suitable_fs.ok_or(FsError::IllegalOperation("Path is not contained in a attached file system".to_string()))
    }
}

trait Attached {
    fn attach_point(&mut self) -> &mut Path;
    fn open_read(&mut self, fd: i64, path: Path) -> FsResult<()>;
    fn open_write(&mut self, fd: i64, path: Path) -> FsResult<()>;
    fn read(&mut self, fd: i64, buffer: &mut [u8]) -> FsResult<usize>;
    fn write(&mut self, fd: i64, buffer: &[u8]) -> FsResult<()>;
    fn seek(&mut self, fd: i64, seek: usize) -> FsResult<()>;
    fn inner_fs_mut(&mut self) -> Box<&mut dyn NonGenericFileSystem>;
}

trait NonGenericFileSystem: BaseFileSystem + ManageFileSystem {}
impl<FS> NonGenericFileSystem for FS where FS: BaseFileSystem + ManageFileSystem {}

struct AttachedFileSystem<T: FunctionalFileSystem> {
    fs: T,
    attach_point: Path,
    files_read: BTreeMap<i64, T::ReadProgress>,
    files_write: BTreeMap<i64, T::WriteProgress>,
}

impl<T: FunctionalFileSystem> AttachedFileSystem<T> {
    fn new(fs: T, attach_point: Path) -> Self {
        Self {
            fs,
            attach_point,
            files_read: BTreeMap::new(),
            files_write: BTreeMap::new(),
        }
    }
}

fn no_such_fd<T>() -> FsResult<T> {
    Err(FsError::IllegalOperation("This file descriptor does not exist".to_string()))
}

impl<T: 'static + FunctionalFileSystem> Attached for AttachedFileSystem<T> {
    fn attach_point(&mut self) -> &mut Path {
        &mut self.attach_point
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

    fn inner_fs_mut(&mut self) -> Box<&mut dyn NonGenericFileSystem> {
        Box::new(&mut self.fs)
    }
}

