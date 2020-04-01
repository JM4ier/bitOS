use crate::fs::*;

use super::*;

pub struct ReadProgress(pub FileProgress);
pub struct WriteProgress(pub FileProgress);

pub struct FileProgress {
    pub sector: u64,
    pub byte_offset: u64,
}

impl FileProgress {
    /// Bytes that have already been read/written in the current sector
    pub fn current_bytes_processed(&self) -> usize {
        (self.byte_offset % SECTOR_SIZE) as usize
    }
}


use crate::fs::*;

#[repr(align(4096))]
pub struct RootSector {
    pub name: [u8; 64],
    pub table_begin: u64,
    pub sectors: u64,
    pub root: u64,
    pub free: u64,
}

impl Default for RootSector {
    fn default() -> Self {
        Self {
            name: [b'0'; 64],
            table_begin: 1,
            sectors: 0,
            root: 0,
            free: 0,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(align(4096))]
pub struct AllocationTable {
    pub entries: [Sector; SECTOR_SIZE as usize / 32],    
}

impl Default for AllocationTable {
    fn default() -> Self {
        Self { 
            entries: [Sector {
                sector_type: SectorType::Reserved,
                size: 0,
                next: 0,
            }; SECTOR_SIZE as usize / 32] 
        }
    }
}

#[derive(Copy, Clone, Default)]
#[repr(align(32))]
pub struct Sector {
    pub sector_type: SectorType,
    pub size: u64,
    pub next: u64, 
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum SectorType {
    /// Unused sector
    Free,
    /// Reserved for special purposes, e.g. the FAT table
    Reserved,
    /// In a data segment that is not the first sector of a file or directory
    Data, 
    /// First sector of a file
    File,
    /// First sector of a directory
    Dir,
}

impl Default for SectorType {
    fn default() -> Self {
        Self::Reserved
    }
}

/// a sector on a disk that stores directory information 
#[derive(Copy, Clone)]
#[repr(align(4096))]
pub struct DirSector {
    pub entries: u64,
    pub data: [u8; SECTOR_SIZE as usize - 8],
}

impl Default for DirSector {
    fn default() -> Self {
        Self {
            entries: 0,
            data: [0u8; SECTOR_SIZE as usize - 8],
        }
    }
}

impl DirSector {
    /// returns a vec of directory entries from the given sector
    // FIXME does not include sector addresses in return vector
    pub fn get_entries(&self) -> Vec<(DirEntry, u64)> {
        let mut entries = Vec::new();
        let mut entry = Vec::new();
        let mut idx = 0;
        for i in 0..self.entries {
            if idx >= self.data.len() {
                panic!("unterminated directory entry");
            }
            if self.data[idx] == 0 {
                entries.push(entry);
                entry = Vec::new();
            } else {
                entry.push(self.data[idx]);
            }
            idx += 1;
        }
        entries
    }

    /// returns a vec of DirSectors built from the directory entries
    /// The directory entries may not contain null `'\0'` characters.
    // FIXME does not put sector addresses in dirsector
    pub fn from_entries(entries: Vec<(DirEntry, u64)>) -> Vec<Self> {
        let max_len = SECTOR_SIZE - 8;
        let entries: Vec<Vec<u8>> = entries.into_iter().map(|mut entry| { entry.push(0); entry }).collect();
        let mut sectors = Vec::new();
        let mut sector = DirSector::default();
        for entry in entries.into_iter() {
            if sector.entries + entry.len() as u64 > max_len {
                sectors.push(sector);
                sector = DirSector::default();
            }
            copy_offset(&entry, &mut sector.data, entry.len(), 0usize, sector.entries as usize);
            sector.entries += entry.len() as u64;
        }
        sectors.push(sector);
        sectors
    }
}

