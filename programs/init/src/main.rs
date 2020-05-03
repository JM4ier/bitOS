#![no_std]
#![no_main]
#![feature(start)]

extern crate alloc;
extern crate bstd;

use bstd::kprint::kprint;
use bstd::fs::{*, file::*, structs::*, path::*};
use alloc::string::*;
use alloc::*;
use alloc::vec::Vec;

type Buffer = [[(u8, u8); 80]; 25];

#[start]
#[no_mangle]
fn _start() {
    main();
    loop {}
}

fn main() {
    kprint(&String::from("Welcome to userspace\n"));
    let path = Path::new("/home/anon/message").unwrap();
    let file = File::<Read>::open(&path);
    let mut file = match file {
        Err(err) => {
            kprint(&format!("{:?}", err));
            return;
        },
        Ok(file) => file,
    };
    let mut buffer = vec![0u8; 4096];
    match file.read(&mut buffer) {
        Ok(bytes) => 
            kprint(&String::from_utf8(buffer[..bytes].to_vec()).unwrap()),
        Err(err) => 
            kprint(&format!("{:?}", err)),
    };
}

