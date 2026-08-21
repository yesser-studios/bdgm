use hadris_udf::{UdfDir, UdfVolume};
use std::{
    fs::{self, File},
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

    for sector in 0..sectors {
        drive.read_exact(&mut buffer)?;
        output.write_all(&buffer)?;

        if sector % 1024 == 0 {
            println!(
                "{}/{} sectors ({:.1}%)",
                sector,
                sectors,
                sector as f64 * 100.0 / sectors as f64
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
