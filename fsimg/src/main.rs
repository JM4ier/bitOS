#![feature(const_generics)]

use fs as bit_fs;
use std::fs as std_fs;
use std::path as std_path;

use bit_fs::filesystem::*;
use bit_fs::memory_devices::*;
use std::io::{Read, Write};

use clap::{Arg, App};

fn main() {
    let matches = App::new("FS Image Creator")
        .version("0.0.1")
        .arg(Arg::with_name("directory")
            .short("d")
            .long("directory")
            .help("specifies source directory")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("image")
            .short("i")
            .long("image")
            .help("specifies binary image file")
            .takes_value(true)
            .required(true))
        .get_matches();

    let path = matches.value_of("directory").unwrap();
    let binary = matches.value_of("image").unwrap();
    let size = 256 * 1; // number of fs blocks (4096 bytes each)

    let path = std_path::Path::new(path); 
    assert!(path.exists());
    assert!(path.is_dir());

    let mut disk = vec![0u8; 4096 * size];
    let ram_disk = RamDisk{ data: &mut disk };
    let mut fat = {
        if let Ok(fat) = bit_fs::ffat::FFAT::format(ram_disk) {
            fat
        } else {
            panic!("could not format ram disk")
        }
    };

    create_image(&mut fat, path, bit_fs::path::Path::from_str("/").unwrap());

    let mut image_file = std_fs::File::create(binary).unwrap();

    drop(fat);

    // write fs to image file
    {
        let mut pos = 0;
        while pos < disk.len() {
            let bytes_written = image_file.write(&disk[pos..]).unwrap();
            pos += bytes_written;
        }
    }
}

fn create_image<'a, FS, D, const BS: usize>(disk: &mut FS, path: &std_path::Path, disk_path: bit_fs::path::Path)
where FS: CompleteFileSystem<D, BS>, D: bit_fs::block::BlockDevice<BS> {
    if !disk_path.is_root() {
        // write fs entry
        if path.is_dir() {
            disk.create_dir(disk_path.clone()).unwrap();
        } else {
            disk.create_file(disk_path.clone()).unwrap();
            let mut wp = disk.open_write(disk_path.clone()).unwrap();
            let mut file = std_fs::File::open(path).unwrap();
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();
            disk.write(&mut wp, &contents).unwrap();
        }
    }

    // write child entries
    if path.is_dir() {
        for entry in std_fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = path.file_name().unwrap().to_str().unwrap().as_bytes().to_vec();
            let disk_path = disk_path.concat(name);
            create_image(disk, &path, disk_path);
        }
    }
}

