use hadris_udf::{UdfDir, UdfVolume};
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom},
    path::Path,
};

#[cfg(windows)]
use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
#[cfg(windows)]
use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
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
pub(crate) fn dump_disc(drive: &str, output: &str) -> io::Result<()> {
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

pub(crate) fn extract_udf_dir(
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
