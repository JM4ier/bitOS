use core::ptr;
use alloc::{string::String, vec::Vec};
use x86_64::structures::paging::PageTableFlags;
use xmas_elf::{*, header::*, program::*};
use fs::*;
use crate::{files, memory, serial_println};

/// loads elf specified by path to memory and returns the entry point of the executable
pub fn load_elf(path: String) -> Result<u64, &'static str> {
    let path = Path::from_str(&path).ok_or("Could not create path")?;
    let file = files::read_all(path).map_err(|_| "Failed to read elf file")?;

    let elf = ElfFile::new(&file)?;
    header::sanity_check(&elf)?;

    let entry_point = elf.header.pt2.entry_point();

    let segments = elf.program_iter()
        .map(|seg| match seg {
            ProgramHeader::Ph64(header) => Ok(*header),
            _ => Err("Only 64-Bit ELF files supported"),
        })
    .collect::<Result<Vec<_>, _>>()?;

    for seg in segments {
        memory::map_range_ignore_err(
            seg.virtual_addr,
            seg.virtual_addr + seg.mem_size,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );

        unsafe {
            ptr::copy(file.as_ptr().offset(seg.offset as _), seg.virtual_addr as _, seg.file_size as _);
        }
    }

    Ok(entry_point)
}

