use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    os::windows::io::AsRawHandle,
    path::Path,
};

use windows_sys::Win32::{
    Foundation::HANDLE,
    System::{
        IO::DeviceIoControl,
        Ioctl::{IOCTL_STORAGE_READ_CAPACITY, STORAGE_READ_CAPACITY},
    },
};

use crate::core::error::{ExtractError, Result};

const SECTOR_SIZE: usize = 2048;

pub(super) struct RawDiscReader {
    file: File,
    pos: u64,
    len: u64,
    cache_sector: u64,
    cache: [u8; SECTOR_SIZE],
    cache_valid: bool,
}

impl RawDiscReader {
    /// Opens a raw device and reads its capacity, requiring 2,048-byte sectors.
    ///
    /// Device opening and capacity query failures return extraction I/O errors;
    /// an unexpected sector size returns an extraction I/O error of kind `InvalidData`.
    pub(super) fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(ExtractError::IoError)?;
        let reader = Self::try_from(file).map_err(ExtractError::IoError)?;
        Ok(reader)
    }

    /// Loads one 2,048-byte sector unless it is already cached.
    ///
    /// Propagates seek or complete-sector read errors.
    fn fill_sector(&mut self, sector: u64) -> io::Result<()> {
        if self.cache_valid && self.cache_sector == sector {
            return Ok(());
        }

        self.file
            .seek(SeekFrom::Start(sector * SECTOR_SIZE as u64))?;
        self.file.read_exact(&mut self.cache)?;
        self.cache_sector = sector;
        self.cache_valid = true;

        Ok(())
    }
}

impl TryFrom<File> for RawDiscReader {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let len = disc_size(&file)?;

        Ok(Self {
            file,
            pos: 0,
            len,
            cache_sector: 0,
            cache: [0u8; SECTOR_SIZE],
            cache_valid: false,
        })
    }
}

impl Read for RawDiscReader {
    /// Reads up to the current sector boundary and advances the byte position.
    ///
    /// Returns zero for an empty buffer or at or beyond disc capacity. Sector seek
    /// and read errors propagate.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.pos >= self.len {
            return Ok(0);
        }

        let sector = self.pos / SECTOR_SIZE as u64;
        let offset = (self.pos % SECTOR_SIZE as u64) as usize;
        self.fill_sector(sector)?;

        let remaining = (self.len - self.pos) as usize;
        let size = buf.len().min(SECTOR_SIZE - offset).min(remaining);
        buf[..size].copy_from_slice(&self.cache[offset..offset + size]);
        self.pos += size as u64;

        Ok(size)
    }
}

impl Seek for RawDiscReader {
    /// Sets the logical byte position, allowing positions beyond the disc's end.
    ///
    /// Negative positions and positions exceeding `u64` return `InvalidInput`.
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let new_position = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::Current(offset) => i128::from(self.pos) + i128::from(offset),
            SeekFrom::End(offset) => i128::from(self.len) + i128::from(offset),
        };

        if new_position < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before the start of the disc",
            ));
        }

        self.pos = u64::try_from(new_position).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek past the addressable range",
            )
        })?;

        Ok(self.pos)
    }
}

/// Queries device capacity in bytes and requires 2,048-byte sectors.
///
/// Returns the Windows API error on failure or `InvalidData` for another sector size.
fn disc_size(file: &File) -> io::Result<u64> {
    let mut capacity = STORAGE_READ_CAPACITY {
        Version: std::mem::size_of::<STORAGE_READ_CAPACITY>() as u32,
        Size: std::mem::size_of::<STORAGE_READ_CAPACITY>() as u32,
        BlockLength: 0,
        NumberOfBlocks: 0,
        DiskLength: 0,
    };
    let mut returned = 0u32;

    let ok = unsafe {
        DeviceIoControl(
            file.as_raw_handle() as HANDLE,
            IOCTL_STORAGE_READ_CAPACITY,
            std::ptr::null(),
            0,
            &mut capacity as *mut _ as *mut _,
            std::mem::size_of::<STORAGE_READ_CAPACITY>() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };

    if ok == 0 {
        return Err(io::Error::last_os_error());
    }

    if capacity.BlockLength != SECTOR_SIZE as u32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unexpected sector size: {} bytes", capacity.BlockLength),
        ));
    }

    Ok(capacity.DiskLength as u64)
}
