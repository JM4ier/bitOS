use crate::fs::*;

mod structs;
use structs::*;

pub const SECTOR_SIZE: u64 = 4096;
pub const SECTOR_SIZE_U: usize = SECTOR_SIZE as usize;
const FAT_ENTRIES_PER_SECTOR: u64 = SECTOR_SIZE / 32;

pub struct FFAT<B: BlockDevice> {
    device: B,
}

const ALLOWED_CHARS: &'static [u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-.";


impl<B: BlockDevice> FileSystem<B> for FFAT<B> {
    type ReadProgress = ReadProgress;
    type WriteProgress = WriteProgress;

    fn allowed_chars() -> &'static [u8] {
        ALLOWED_CHARS
    }

    fn mount(device: B) -> FsResult<Self> {
        Ok(Self{device})
    }

    fn format(device: B) -> FsResult<Self> {
        assert!(!device.is_read_only());
        assert!(device.blocksize() == SECTOR_SIZE);        
        assert!(device.blocks() >= 8);

        let mut device = device;

        let sectors = device.blocks();
        assert!(core::mem::size_of::<Sector>() == 32);
        let fat_entry_size = 32;
        let fat_entries_per_sector = SECTOR_SIZE / fat_entry_size;

        let mut fat_sectors = sectors / fat_entries_per_sector;
        if SECTOR_SIZE * fat_sectors < sectors { fat_sectors += 1; } 
        let fat_sectors = fat_sectors;
        let free_sectors = sectors - fat_sectors - 2;

        let mut fat_table = Vec::with_capacity(sectors as usize);

        let reserved_fat_entry = Sector {
            sector_type: SectorType::Reserved, 
            size: 0,
            next: 0,
        };

        // push a reserved entry for each fat-table sector and one for the root sector
        for _ in 0..(fat_sectors+1) {
            fat_table.push(reserved_fat_entry);
        }

        // push root entry
        fat_table.push(Sector {
            sector_type: SectorType::Dir,
            size: 0,
            next: 0,
        });

        // push free entries
        let free_offset = fat_table.len();
        for i in 0..free_sectors {
            let next = if i == free_sectors - 1 { 0 } else { i+1 };
            fat_table.push(Sector {
                sector_type: SectorType::Free,
                size: 0,
                next,
            });
        }

        // pad fat table entries with reserved sectors (which are outside of the device)
        while fat_table.len() % (SECTOR_SIZE as usize / 32usize) > 0 {
            fat_table.push(reserved_fat_entry);
        }

        // write the fat table to the device
        for i in 0..fat_sectors {
            let mut table = device.get_mut::<_, AllocationTable>(1 + i as u64)?;
            copy_offset(&fat_table, &mut table.entries, fat_entries_per_sector as usize, (i * fat_entries_per_sector) as usize, 0);
            table.write()?;
        }

        // write the root sector to the device
        let root_sector = RootSector {
            name: [b'X'; 64],
            table_begin: 1,
            sectors,
            root: 1 + fat_sectors,
            free: 2 + fat_sectors,
        };
        device.write(0u64, &root_sector)?;

        let root = [0u8; SECTOR_SIZE as usize];
        device.write(root_sector.root, &root)?;

        Ok(Self{device})
    }

    fn is_read_only(&self) -> bool {
        self.device.is_read_only()
    }

    fn create_file(&mut self, path: Path) -> FsResult<()> {
        let meta = Sector {
            sector_type: SectorType::File,
            size: 0, 
            next: 0,
        };
        self.create(&path, meta)?;
        Ok(())
    }

    fn create_dir(&mut self, path: Path) -> FsResult<()> {
        let meta = Sector {
            sector_type: SectorType::Dir,
            size: SECTOR_SIZE, 
            next: 0,
        };
        let addr = self.create(&path, meta)?;

        // write an empty list of directory entries to the sector
        let dir_entries = DirData::new();
        let buffer = raw_dir_data(&dir_entries);
        self.device.write(addr, &buffer[0])?;

        Ok(())
    }

    fn exists_file(&mut self, path: Path) -> FsResult<bool> {
        self.exists(&path, SectorType::File)
    }

    fn exists_dir(&mut self, path: Path) -> FsResult<bool> {
        self.exists(&path, SectorType::Dir)
    }

    fn read_dir(&mut self, path: Path) -> FsResult<Vec<Filename>> {
        let addr = self.walk(&path)?;
        let dirdata = self.read_dir_at_addr(addr)?;
        let entries = dirdata.into_iter().map(|x| x.1).collect();
        Ok(entries)
    }

    fn open_write(&mut self, path: Path) -> FsResult<WriteProgress> {
        Ok(WriteProgress(self.open(&path)?))
    }

    fn open_read(&mut self, path: Path) -> FsResult<ReadProgress> {
        Ok(ReadProgress(self.open(&path)?))
    }

    fn write(&mut self, progress: &mut WriteProgress, buffer: &[u8]) -> FsResult<()> {
        // TODO
        panic!("not implemented");
    }

    fn read(&mut self, progress: &mut ReadProgress, buffer: &mut [u8]) -> FsResult<u64> {
        let progress = &mut progress.0;

        if progress.sector == 0 {
            return Ok(0u64);
        }

        let mut buffer_idx = 0;

        let mut buf = [0u8; SECTOR_SIZE_U];

        while buffer_idx < buffer.len() {
            let bytes_from_sector_start = progress.byte_offset as usize % SECTOR_SIZE_U;
            let bytes_to_sector_end = SECTOR_SIZE_U - bytes_from_sector_start;
            let bytes_to_buffer_end = buffer.len() - buffer_idx;
            let read_bytes = bytes_to_sector_end.min(bytes_to_buffer_end);

            self.device.read(progress.sector, &mut buf)?;
            copy_offset(&buf, buffer, read_bytes, bytes_from_sector_start, buffer_idx);

            buffer_idx += read_bytes;

            let offset = progress.byte_offset as usize;
            if offset / SECTOR_SIZE_U < (offset + read_bytes) / SECTOR_SIZE_U {
                // arrived at next sector
                if let Some(next) = self.next_sector(progress.sector)? {
                    // there is a next sector
                    progress.sector = next;
                } else {
                    // at end of file, return number of read bytes
                    progress.sector = 0;
                    return Ok(buffer_idx as u64);
                }
            }

            progress.byte_offset += read_bytes as u64;
        }

        Ok(buffer_idx as u64)
    }

    fn seek(&mut self, progress: &mut ReadProgress, seeking: u64) -> FsResult<()> {
        progress.0.byte_offset += seeking;
        Ok(())
    }

    fn delete(&mut self, path: Path) -> FsResult<()> {
        let addr = self.walk(&path)?;
        let meta = self.read_sector_meta(addr)?;

        if let SectorType::Dir = meta.sector_type {
            self.delete_children(&path)?;
        }
        
        let name = if let Some(name) = path.name() {
            name
        } else {
            return Err(FsError::IllegalOperation);
        };
        let parent_dir = if let Some(parent_dir) = path.parent_dir() {
            parent_dir
        } else {
            return Err(FsError::IllegalOperation);
        };

        let parent_addr = self.walk(&parent_dir)?;
        let dir_data = self.read_dir_at_addr(parent_addr)?;

        // find childs address
        let child_addr = dir_data.iter().filter(|entry| entry.1 == name).last();

        if let Some((addr, _)) = child_addr {
            // free child sectors
            self.free_sectors(*addr)?;

            // remove child entry from parent
            let dir_data = dir_data.into_iter().filter(|entry| entry.1 != name).collect();
            self.write_dir_at_addr(parent_addr, &dir_data)?;
        }

        Ok(())
    }

    fn clear(&mut self, path: Path) -> FsResult<()> {
        let addr = self.walk(&path)?;
        let meta = self.read_sector_meta(addr)?;

        if let SectorType::Dir = meta.sector_type {
            self.delete_children(&path)?;
        }

        match meta.sector_type {
            SectorType::Dir => {
                self.clear_at_addr(addr)?;
                let dir_data = DirData::new();
                self.write_dir_at_addr(addr, &dir_data)?;
            },
            SectorType::File => {
                self.clear_at_addr(addr)?;
            },
            SectorType::Data | 
            SectorType::Free | 
            SectorType::Reserved => return Err(FsError::IllegalOperation),
        }

        Ok(())
    }

}

impl<B: BlockDevice> FFAT<B> {

    fn read_sector_meta(&mut self, addr: u64) -> FsResult<Sector> {
        if let Some((table_addr, table_idx)) = self.sector_to_table_location(addr) {
            let table = self.device.get::<_, AllocationTable>(table_addr)?;
            Ok(table.entries[table_idx as usize])
        } else {
            Err(FsError::IllegalOperation)
        }
    }

    fn write_sector_meta(&mut self, addr: u64, meta: Sector) -> FsResult<()> {
        if let Some((table_addr, table_idx)) = self.sector_to_table_location(addr) {
            let mut table = self.device.get_mut::<_, AllocationTable>(table_addr)?;
            table.entries[table_idx as usize] = meta;
            table.write()?;
            Ok(())
        } else {
            Err(FsError::IllegalOperation)
        }
    }


    /// walks the path and returns the address of the target file or directory
    fn walk_from(&mut self, addr: u64, path: &Path) -> FsResult<u64> {
        let (head, tail) = path.clone().head_tail();
        if let Some(head) = head {
            let dir_data = self.read_dir_at_addr(addr)?; 

            let mut next_addr = None;
            for entry in dir_data {
                if entry.1 == head {
                    next_addr = Some(entry.0);
                    break;
                }
            }
            if let Some(a) = next_addr {
                self.walk_from(a, &tail)
            } else {
                Err(FsError::FileNotFound)
            }
        } else {
            Ok(addr)
        }
    }

    /// walks the path and starts from the root directory
    fn walk(&mut self, path:  &Path) -> FsResult<u64> {
        let fs_root = self.root_sector()?.root;
        self.walk_from(fs_root, path)
    }

    /// gets a free sector and returns it
    fn allocate_sector(&mut self) -> FsResult<u64> {
        let mut root_sector = self.device.get::<_, RootSector>(0u64)?;
        let addr = root_sector.free;
        let next = self.next_sector(addr)?;
        if let Some(next) = next {
            root_sector.free = next;
        } else {
            return Err(FsError::NotEnoughSpace);
        }

        // change metadata
        self.write_sector_meta(addr, Sector { 
            sector_type: SectorType::Free, 
            size: 0, 
            next: 0 
        })?;

        self.device.write(0u64, &root_sector)?;
        Ok(addr)
    }

    /// frees the linkage of sectors beginning at addr
    fn free_sectors(&mut self, addr: u64) -> FsResult<()> {
        let mut root_sector = self.root_sector()?;
        let mut end_addr = addr;
        loop {
            let mut meta = self.read_sector_meta(end_addr)?;
            meta.sector_type = SectorType::Free;
            meta.size = 0;
            self.write_sector_meta(end_addr, meta)?;

            if let Some(next) = self.next_sector(end_addr)? {
                end_addr = next;
            } else {
                break;
            }
        }

        let mut end_meta = self.read_sector_meta(end_addr)?;
        end_meta.next = root_sector.free;
        self.write_sector_meta(end_addr, end_meta)?;

        root_sector.free = end_addr;
        self.device.write(0u64, &root_sector)?;

        Ok(())
    }

    /// reads directory at address
    fn read_dir_at_addr(&mut self, addr: u64) -> FsResult<DirData> {
        let entry = self.read_sector_meta(addr)?;
        if entry.sector_type == SectorType::Dir {
            let sectors = (entry.size as usize + SECTOR_SIZE_U - 1) / SECTOR_SIZE_U;
            let mut buffers = vec![[0u8; SECTOR_SIZE_U]; sectors];

            let mut addr = addr;
            for i in 0..sectors {
                self.device.read(addr, &mut buffers[i])?;
                if let Some(a) = self.next_sector(addr)? {
                    addr = a;
                } else {
                    return Err(FsError::InternalError);
                }
            }

            let dir_data = dir_data_from_raw(&buffers);
            Ok(dir_data)
        } else {
            Err(FsError::IllegalOperation)
        }
    }

    /// clears the given file or directory from disk
    fn clear_at_addr(&mut self, addr: u64) -> FsResult<()> {
        if let Some(tail) = self.next_sector(addr)? {
            self.free_sectors(tail)?;
        }

        let mut meta = self.read_sector_meta(addr)?;
        meta.size = 0;
        self.write_sector_meta(addr, meta)?;
        Ok(())
    }

    /// writes directory data at specified address
    fn write_dir_at_addr(&mut self, addr: u64, dir_data: &DirData) -> FsResult<()> {
        let raw_data = raw_dir_data(&dir_data);
        let mut addr = addr;
        for raw in raw_data {
            self.device.write(addr, &raw)?;

            // get next address 
            let next = if let Some(addr) = self.next_sector(addr)? {
                addr
            } else {
                self.allocate_sector()?
            };
            let mut meta = self.read_sector_meta(addr)?;
            meta.next = next;
            self.write_sector_meta(addr, meta)?;
            addr = next;
        }

        // if there are some unwritten sectors left, free them
        if let Some(next) = self.next_sector(addr)? {
            self.free_sectors(next)?;
        }

        Ok(())
    }

    /// returns the next sector of the specified sector
    fn next_sector(&mut self, sector: u64) -> FsResult<Option<u64>> {
        let next = self.read_sector_meta(sector)?.next;

        if next <= 0 {
            Ok(None)
        } else {
            Ok(Some(next))
        }
    }

    /// returns the sector in which the table entry for the given sector lies
    /// and the index inside that table
    fn sector_to_table_location(&mut self, sector: u64) -> Option<(u64, u64)> {
        if let Ok(root_sector) = self.root_sector() {
            if sector < root_sector.root || sector >= root_sector.sectors {
                None
            } else {
                Some((sector / FAT_ENTRIES_PER_SECTOR + root_sector.table_begin, sector % FAT_ENTRIES_PER_SECTOR))
            }
        } else {
            None
        }
    }

    fn root_sector(&mut self) -> FsResult<RootSector> {
        let mut root_sector = RootSector::default();
        self.device.read(0u64, &mut root_sector)?;
        Ok(root_sector)
    }

    fn exists(&mut self, path: &Path, sector_type: SectorType) -> FsResult<bool> {
        match self.walk(path) {
            Err(FsError::FileNotFound) => Ok(false),
            Ok(addr) => {
                let meta = self.read_sector_meta(addr)?;
                Ok(meta.sector_type == sector_type)
            },
            Err(err) => Err(err),
        }
    }

    fn create(&mut self, path: &Path, meta: Sector) -> FsResult<u64> {
        if let Some(parent) = path.parent_dir() {
            let parent_addr = self.walk(&parent)?;
            let mut dir_data = self.read_dir_at_addr(parent_addr)?;

            let filename = match path.name() {
                Some(name) => name,
                None => return Err(FsError::IllegalOperation),
            };

            for dir_entry in dir_data.iter() {
                if dir_entry.1 == filename {
                    return Err(FsError::IllegalOperation);
                }
            }

            // get a free sector
            let file_addr = self.allocate_sector()?;
            dir_data.push((file_addr, filename));

            // write file/directory metadata
            self.write_sector_meta(file_addr, meta)?;

            // write directory data
            self.write_dir_at_addr(parent_addr, &dir_data)?;

            Ok(file_addr)
        } else {
            Err(FsError::IllegalOperation)
        }
    }

    /// opens a file and returns a fileprogress to it
    /// returns err if an underlying read operation failed
    /// or the path refers to a directory
    fn open(&mut self, path: &Path) -> FsResult<FileProgress> {
        let addr = self.walk(path)?;
        match self.read_sector_meta(addr)?.sector_type {
            SectorType::File => 
                Ok(FileProgress {
                    byte_offset: 0,
                    sector: addr,
                }),
            _ => Err(FsError::IllegalOperation),
        }
    }

    /// deletes all child elements of this directory
    fn delete_children(&mut self, path: &Path) -> FsResult<()> {
        let addr = self.walk(path)?;
        let dir_data = self.read_dir_at_addr(addr)?;
        let children: Vec<Path> = dir_data.into_iter().map(|entry| path.concat(entry.1)).collect();
        for child in children {
            self.delete(child)?;
        }
        Ok(())
    }
}

