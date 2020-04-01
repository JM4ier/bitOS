use crate::fs::*;

mod structs;
use structs::*;

pub const SECTOR_SIZE: u64 = 4096;
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
        for i in 0..(fat_sectors+1) {
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
        panic!("not implemented");
    }

    fn create_dir(&mut self, path: Path) -> FsResult<()> {
        panic!("not implemented");
    }

    fn exists_file(&mut self, path: Path) -> FsResult<bool> {
        panic!("not implemented");
    }

    fn exists_dir(&mut self, path: Path) -> FsResult<bool> {
        panic!("not implemented");
    }

    fn read_dir(&mut self, path: Path) -> FsResult<Vec<DirEntry>> {
        panic!("not implemented");
    }

    fn open_write(&mut self, path: Path) -> FsResult<WriteProgress> {
        panic!("not implemented");
    }

    fn open_read(&mut self, path: Path) -> FsResult<ReadProgress> {
        panic!("not implemented");
    }

    fn write(&mut self, progress: &mut WriteProgress, buffer: &[u8]) -> FsResult<()> {
        assert!(buffer.len() % SECTOR_SIZE as usize == 0);
        self._write(&mut progress.0, buffer)
    }

    fn read(&mut self, progress: &mut ReadProgress, buffer: &mut [u8]) -> FsResult<u64> {
        assert!(buffer.len() % SECTOR_SIZE as usize == 0);
        self._read(&mut progress.0, buffer)
    }

    fn seek(&mut self, progress: &mut ReadProgress, seeking: u64) -> FsResult<()> {
        progress.0.byte_offset += seeking;
        Ok(())
    }

    fn delete(&mut self, path: Path) -> FsResult<()> {
        panic!("not implemented");
    }

    fn clear(&mut self, path: Path) -> FsResult<()> {
        panic!("not implemented");
    }

}

impl<B: BlockDevice> FFAT<B> {
    fn _read(&mut self, progress: &mut FileProgress, buf: &mut [u8]) -> FsResult<u64> {
        let mut bufidx = 0; 
        let mut sector_buffer = [0u8; SECTOR_SIZE as usize];

        while bufidx < buf.len() {
            // bytes that can be read from the current sector
            let bytes_in_sector = SECTOR_SIZE as usize - progress.current_bytes_processed();

            // bytes that should be read from the current sector
            let bytes_to_read = bytes_in_sector.min(buf.len() as usize - bufidx);

            self.device.read(progress.sector, &mut sector_buffer)?;

            // copy bytes to destination buffer
            copy_offset(&sector_buffer, buf, bytes_to_read, progress.current_bytes_processed(), bufidx);

            bufidx += bytes_to_read;
            progress.byte_offset += bytes_to_read as u64;

            // check if reading past a sector border and finding the appropriate next sector
            if progress.byte_offset >= SECTOR_SIZE {
                assert!(progress.byte_offset == SECTOR_SIZE);
                progress.byte_offset = 0;
                
                // read FAT
                if let Some((table_sector, table_idx)) = self.sector_to_table_location(progress.sector) {
                    let mut table = AllocationTable::default();
                    self.device.read(table_sector, &mut table)?;
                    progress.sector = table.entries[table_idx as usize].next;
                    if let None = self.sector_to_table_location(progress.sector) {
                        // end of file
                        break;
                    }
                } else {
                    panic!("Just read a reserved sector instead of a file");
                }
            }
        }
        Ok(bufidx as u64)
    }

    fn _write(&mut self, progress: &mut FileProgress, buf: &[u8]) -> FsResult<()> {
        let mut bufidx = 0;
        
        while bufidx < buf.len() {
            // bytes that can still be written to the same sector
            let bytes_in_sector = SECTOR_SIZE as usize - progress.current_bytes_processed();

            // bytes to be written
            let bytes_to_write = bytes_in_sector.min(buf.len() - bufidx as usize);

            let mut sector_buffer = [0u8; SECTOR_SIZE as usize];
            copy_offset(&buf, &mut sector_buffer, bytes_to_write, bufidx, progress.current_bytes_processed());

            self.device.write(progress.sector, &sector_buffer)?;

            bufidx += bytes_to_write as usize;
            progress.byte_offset += bytes_to_write as u64;

            if progress.byte_offset >= SECTOR_SIZE {
                assert_eq!(progress.byte_offset, SECTOR_SIZE);
                progress.byte_offset = 0;

                let root_sector = self.root_sector()?;

                if let Some((table_sector, table_idx)) = self.sector_to_table_location(root_sector.free) {
                    let next = root_sector.free;

                    let mut table = AllocationTable::default();
                    self.device.read(progress.sector, &mut table)?;

                    table.entries[table_idx as usize].next = next;
                    table.entries[table_idx as usize].sector_type = SectorType::Data;

                    self.device.write(progress.sector, &table)?;

                    progress.sector = next;

                    if let Some((next_table_sector, next_table_idx)) = self.sector_to_table_location(next) {
                        self.device.read(next, &mut table)?;
                        table.entries[next_table_idx as usize].next = 0;
                        table.entries[next_table_idx as usize].sector_type = SectorType::Data;
                        self.device.write(next, &table)?;
                    } else {
                        return Err(FsError::NotEnoughSpace);
                    }
                } else {
                    return Err(FsError::NotEnoughSpace);
                }
                
            }

        }
        Ok(())
    }

    /// returns the sector in which the table entry for the given sector lies
    /// and the index inside that table
    fn sector_to_table_location(&mut self, sector: u64) -> Option<(u64, u64)> {
        if let Ok(root_sector) = self.root_sector() {
            if sector < root_sector.root || sector >= root_sector.sectors {
                None
            } else {
                Some((sector / FAT_ENTRIES_PER_SECTOR, sector % FAT_ENTRIES_PER_SECTOR))
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
}

