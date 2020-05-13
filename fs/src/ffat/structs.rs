use super::*;

use bytevec::*;

pub struct ReadProgress(pub FileProgress, pub usize);
pub struct WriteProgress(pub FileProgress);

pub struct FileProgress {
    /// begin of file that stores the files metadata
    pub head: usize,
    
    /// current sector where data is being read from or written to
    pub sector: usize,

    /// offset from begin of file
    pub byte_offset: usize,
}

impl FileProgress {
    /// Bytes that have already been read/written in the current sector
    pub fn current_bytes_processed(&self) -> usize {
        (self.byte_offset % BLOCK_SIZE) as usize
    }
}


#[repr(align(4096))]
pub struct RootSector {
    pub name: [u8; 64],
    pub table_begin: usize,
    pub sectors: usize,
    pub root: usize,
    pub free: usize,
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
    pub entries: [Sector; BLOCK_SIZE / 32],    
}

impl Default for AllocationTable {
    fn default() -> Self {
        Self { 
            entries: [Sector {
                sector_type: SectorType::Reserved,
                size: 0,
                next: 0,
            }; BLOCK_SIZE / 32] 
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
#[repr(align(32))]
pub struct Sector {
    pub sector_type: SectorType,
    pub size: usize,
    pub next: usize, 
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
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

pub type DirEntry = (usize, Filename);
pub type DirData = Vec<DirEntry>;

pub fn raw_dir_data(data: &DirData) -> (Vec<[u8; BLOCK_SIZE]>, usize) {
    let bytes = data.encode::<u64>().unwrap();
    let sectors = (bytes.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
    let mut raw = Vec::with_capacity(sectors as usize);

    let mut bytes_processed = 0;
    for _ in 0..sectors {
        let mut sector = [0u8; BLOCK_SIZE];
        let bytes_to_copy = (bytes.len() - bytes_processed).min(BLOCK_SIZE);
        copy_offset(&bytes, &mut sector, bytes_to_copy, bytes_processed, 0);
        bytes_processed += bytes_to_copy;
        raw.push(sector);
    }
    (raw, bytes.len() as usize)
}

pub fn dir_data_from_raw(raw: &Vec<[u8; BLOCK_SIZE]>, size: usize) -> DirData {
    let size = size as usize;
    let raw: Vec<u8> = raw.iter().flat_map(|sector| sector.iter()).map(|v| *v).collect();
    let data = DirData::decode::<u64>(&raw[..size]).unwrap();
    data
}

