use hadris_udf::{UdfDir, UdfVolume};
use std::{
    fs::{self, File},
    path::Path,
};
#[cfg(windows)]
use bdgm::error::BDGMError;

#[cfg(windows)]
use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
#[cfg(windows)]
use std::{
    fs::OpenOptions,
    io::{self, Read, Seek, SeekFrom, Write},
};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::HANDLE,
    System::{
        IO::DeviceIoControl,
        Ioctl::{IOCTL_STORAGE_READ_CAPACITY, STORAGE_READ_CAPACITY},
    },
};

#[cfg(windows)]
const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
#[cfg(windows)]
const SECTOR_SIZE: usize = 2048;

#[cfg(windows)]
fn read_sector<R: Read + Seek>(
    r: &mut R,
    sector: u64,
    buf: &mut [u8; SECTOR_SIZE],
) -> io::Result<()> {
    r.seek(SeekFrom::Start(sector * SECTOR_SIZE as u64))?;
    r.read_exact(buf)
}

#[cfg(windows)]
fn tag_id(sector: &[u8]) -> u16 {
    u16::from_le_bytes([sector[0], sector[1]])
}

/// Returns the exclusive end sector of the UDF partition(s) — i.e. the
/// last sector you actually need to copy off the disc.
#[cfg(windows)]
fn find_udf_end_sector<R: Read + Seek>(drive: &mut R) -> io::Result<u64> {
    let mut buf = [0u8; SECTOR_SIZE];

    // Primary Anchor Volume Descriptor Pointer is always at sector 256.
    read_sector(drive, 256, &mut buf)?;
    if tag_id(&buf) != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "no AVDP at sector 256",
        ));
    }

    let mvds_len = u32::from_le_bytes(buf[16..20].try_into().unwrap()) as u64; // bytes
    let mvds_loc = u32::from_le_bytes(buf[20..24].try_into().unwrap()) as u64; // sector
    let mvds_sectors = mvds_len.div_ceil(SECTOR_SIZE as u64);

    let mut max_end: u64 = 0;
    for i in 0..mvds_sectors {
        read_sector(drive, mvds_loc + i, &mut buf)?;
        match tag_id(&buf) {
            5 => {
                // Partition Descriptor: start (u32 @188) + length (u32 @192), both in sectors
                let start = u32::from_le_bytes(buf[188..192].try_into().unwrap()) as u64;
                let len = u32::from_le_bytes(buf[192..196].try_into().unwrap()) as u64;
                max_end = max_end.max(start + len);
            }
            8 | 0 => break, // Terminating Descriptor (or blank/unused) ends the sequence
            _ => {}
        }
    }

    Ok(max_end)
}

#[cfg(windows)]
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

#[cfg(windows)]
pub fn dump_disc_with_progress(
    drive: &str,
    output: &str,
    mut on_progress: impl FnMut(u64, u64),
) -> io::Result<()> {
    let mut drive = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_NO_BUFFERING)
        .open(drive)?;

    let size = disc_size(&drive)?;

    println!(
        "Disc size: {} bytes ({:.2} GiB)",
        size,
        size as f64 / 1024.0 / 1024.0 / 1024.0
    );

    if size % SECTOR_SIZE as u64 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "disc size is not a multiple of 2048 bytes",
        ));
    }

    let mut output = File::create(output)?;

    // 2048-byte aligned buffer.
    let mut buffer = [0u8; SECTOR_SIZE];

    let sectors = size / SECTOR_SIZE as u64;

    let udf_end = find_udf_end_sector(&mut drive).unwrap_or_else(|e| {
        eprintln!("{}; dumping whole disc", e);
        sectors
    });
    let sectors_to_copy = udf_end.min(sectors);
    drive.seek(SeekFrom::Start(0))?;

    println!(
        "Dumped partition size: {} bytes ({:.2} MiB)",
        sectors_to_copy * SECTOR_SIZE as u64,
        (sectors_to_copy * SECTOR_SIZE as u64) as f64 / 1024.0 / 1024.0
    );

    for sector in 0..sectors_to_copy {
        drive.read_exact(&mut buffer)?;
        output.write_all(&buffer)?;

        if sector % 512 == 0 || sector + 1 == sectors_to_copy {
            on_progress(sector + 1, sectors_to_copy);
        }

        if sector % 1024 == 0 {
            println!(
                "{}/{} sectors ({:.1}%)",
                sector,
                sectors_to_copy,
                sector as f64 * 100.0 / sectors_to_copy as f64
            );
        }
    }

    output.flush()?;

    Ok(())
}

#[cfg(windows)]
pub fn dump_disc(drive: &str, output: &str) -> io::Result<()> {
    dump_disc_with_progress(drive, output, |_, _| {})
}

/// Sector-aligned reader for raw optical drives (`\\.\X:`).
///
/// Windows optical drives only serve 2048-byte aligned reads (an unaligned
/// 16-byte read fails with OS error 1), while `hadris-udf` parses
/// descriptors with small unaligned reads. This adapter translates any
/// `Read`/`Seek` into whole-sector reads with a one-sector cache, so UDF
/// metadata and a single small file can be fetched with only dozens of
/// sectors touched instead of a full dump.
#[cfg(windows)]
struct RawSectorReader {
    file: File,
    pos: u64,
    len: u64,
    cache_sector: u64,
    cache: [u8; SECTOR_SIZE],
    cache_valid: bool,
}

#[cfg(windows)]
impl RawSectorReader {
    fn new(file: File) -> io::Result<Self> {
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

    fn fill_sector(&mut self, sector: u64) -> io::Result<()> {
        if self.cache_valid && self.cache_sector == sector {
            return Ok(());
        }
        self.file.seek(SeekFrom::Start(sector * SECTOR_SIZE as u64))?;
        self.file.read_exact(&mut self.cache)?;
        self.cache_sector = sector;
        self.cache_valid = true;
        Ok(())
    }
}

#[cfg(windows)]
impl Read for RawSectorReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let sector = self.pos / SECTOR_SIZE as u64;
        let offset = (self.pos % SECTOR_SIZE as u64) as usize;
        self.fill_sector(sector)?;
        let n = buf.len().min(SECTOR_SIZE - offset);
        buf[..n].copy_from_slice(&self.cache[offset..offset + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

#[cfg(windows)]
impl Seek for RawSectorReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new = match pos {
            SeekFrom::Start(o) => o as i64,
            SeekFrom::Current(o) => self.pos as i64 + o,
            SeekFrom::End(o) => self.len as i64 + o,
        };
        if new < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before start of disc",
            ));
        }
        self.pos = new as u64;
        Ok(self.pos)
    }
}

/// Opens a raw disc (`\\.\X:`) as a seekable UDF volume.
///
/// Uses [`RawSectorReader`] over a plain handle (no `FILE_FLAG_NO_BUFFERING`),
/// so only the sectors needed for descriptor/directory/file lookups are
/// touched.
#[cfg(windows)]
fn open_raw_udf(drive: &str) -> anyhow::Result<UdfVolume<RawSectorReader>> {
    Ok(UdfVolume::open(RawSectorReader::new(File::open(drive)?)?)?)
}

/// Fs-map-only probe: checks the UDF directory structure for
/// `/BDGM/DISC.BDGM` without reading any file contents.
///
/// Returns `Ok(false)` when the `BDGM` dir is missing, is not a directory,
/// or contains no `DISC.BDGM` file entry. Any I/O/UDF parse error is
/// returned as `Err` so the caller can fall back to a full dump.
#[cfg(windows)]
pub fn raw_disc_has_manifest(drive: &str) -> anyhow::Result<bool> {
    let udf = open_raw_udf(drive)?;
    let root = udf.root_dir()?;
    let Some(bdgm) = root.find("BDGM") else {
        return Ok(false);
    };
    if !bdgm.is_dir() {
        return Ok(false);
    }
    let bdgm_dir = udf.read_directory(&bdgm.icb)?;
    Ok(bdgm_dir
        .find("DISC.BDGM")
        .is_some_and(|entry| entry.is_file()))
}

/// Reads only `/BDGM/DISC.BDGM` off a raw disc via seek-based partial UDF
/// reads (descriptors + directory chain + the manifest's own extents).
///
/// The fs map is checked first: missing `BDGM` dir or `DISC.BDGM` entry
/// yields `BDGMError::BDGMDirectoryMissing` / `BDGMError::DiscFileMissing`
/// without touching any other file data.
#[cfg(windows)]
pub fn read_manifest_from_raw_disc(drive: &str) -> anyhow::Result<String> {
    let udf = open_raw_udf(drive)?;
    let root = udf.root_dir()?;
    let bdgm = root
        .find("BDGM")
        .ok_or(BDGMError::BDGMDirectoryMissing)?;
    if !bdgm.is_dir() {
        return Err(BDGMError::BDGMDirectoryMissing.into());
    }
    let bdgm_dir = udf.read_directory(&bdgm.icb)?;
    let entry = bdgm_dir
        .find("DISC.BDGM")
        .ok_or(BDGMError::DiscFileMissing)?;
    if !entry.is_file() {
        return Err(BDGMError::DiscFileMissing.into());
    }
    let bytes = udf.read_file(entry)?;
    Ok(String::from_utf8(bytes).map_err(|_| BDGMError::DiscFileInvalid)?)
}

pub fn extract_udf_dir(
    udf: &UdfVolume<File>,
    dir: &UdfDir,
    output_path: &Path,
) -> anyhow::Result<()> {
    for entry in dir.entries() {
        if entry.is_parent() {
            continue;
        }

        let name = entry.name();
        if entry.is_dir() {
            let child_path = output_path.join(name);
            fs::create_dir_all(&child_path)?;
            let child = udf.read_directory(&entry.icb)?;
            extract_udf_dir(udf, &child, &child_path)?;
        } else {
            let file_path = output_path.join(name);
            let bytes = udf.read_file(entry)?;
            fs::write(&file_path, bytes)?;
        }
    }

    Ok(())
}
