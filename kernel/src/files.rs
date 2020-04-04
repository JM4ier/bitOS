use alloc::{vec, string::String};
use lazy_static::lazy_static;
use spin::Mutex;
use crate::{print, println, serial_println, fs::{*, ffat::*}};


static DISK_IMAGE: &'static [u8] = include_bytes!("../../disk.img");

lazy_static! {
    // copy the entire read-only DISK_IMAGE to be able to write to it
    // does not persist accross reboots
    pub static ref DISK: Mutex<FFAT<OwnedDisk>> = {
        let mut img = vec![[0u8; 4096]; DISK_IMAGE.len() / 4096];
        for i in 0..img.len() {
            for k in 0..4096 {
                img[i][k] = DISK_IMAGE[4096*i+k];
            }
        }
        Mutex::new(FFAT::mount(OwnedDisk::new(img)).unwrap())
    };
}

/// initializes the file system if it isn't already
pub fn init() {
    // do something with the DISK so that it gets initialized
    DISK.lock();
}

/// reads a file from /home/anon/message and displays it in the vga buffer
pub fn message() {
    let mut disk = DISK.lock();
    let mut buf = [0u8; 4096];
    let path = Path::from_str::<FFAT<OwnedDisk>, OwnedDisk>("/home/anon/message").unwrap();
    let mut handle = disk.open_read(path).unwrap();
    let bytes = disk.read(&mut handle, &mut buf).unwrap() as usize;
    serial_println!("read {} bytes from /home/anon/message", bytes);
    for &byte in buf[..bytes].iter() {
        print!("{}", byte as char);
    }
}

