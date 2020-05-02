use alloc::{vec, vec::Vec};
use spin::{Once, Mutex};
use crate::{print, fs::{*, ffat::*}};
use core::ops::DerefMut;
use fs::FsResult;


static DISK_IMAGE: &'static [u8] = include_bytes!("../../disk.img");

static mut DISK: Once<Mutex<FFAT<'static>>> = Once::new();

/// initializes the file system if it isn't already
pub fn init() {
    // do something with the DISK so that it gets initialized
    fs();
}

/// reads a file from /home/anon/message and displays it in the vga buffer
pub fn message() {
    let msg = read_all(Path::from_str("/home/anon/message").unwrap()).unwrap();
    for &byte in msg.iter() {
        print!("{}", byte as char);
    }
}

/// returns an exclusive handle to the file system
/// which is lazily initialized
pub fn fs() -> impl DerefMut<Target = impl fs::FileSystem<'static>> {
    // copy the entire read-only DISK_IMAGE to be able to write to it
    // does not persist accross reboots
    unsafe {
        DISK.call_once(|| { 
            let mut img = vec![[0u8; 4096]; DISK_IMAGE.len() / 4096];
            for i in 0..img.len() {
                for k in 0..4096 {
                    img[i][k] = DISK_IMAGE[4096*i+k];
                }
            }
            Mutex::new(FFAT::mount(OwnedDisk::new(img)).unwrap())
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
        let bytes_read = fs.read(&mut handle, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        vec.append(&mut buffer[..bytes_read as usize].to_vec());
    }
    Ok(vec)
}

