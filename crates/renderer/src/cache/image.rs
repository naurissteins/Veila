use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

use super::atomic::write_atomic;
use crate::FrameSize;

pub(crate) fn read_pixels(
    path: &Path,
    magic: &[u8; 8],
    max_dimension: u32,
    expected: Option<FrameSize>,
) -> Option<(FrameSize, Vec<u8>)> {
    let mut file = fs::File::open(path).ok()?;
    let size = read_header(&mut file, magic, max_dimension, expected)?;
    let byte_len = size.byte_len()?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(byte_len).ok()?;
    pixels.resize(byte_len, 0);
    file.read_exact(&mut pixels).ok()?;
    Some((size, pixels))
}

pub(crate) fn cached_size(path: &Path, magic: &[u8; 8], max_dimension: u32) -> Option<FrameSize> {
    read_header(&mut fs::File::open(path).ok()?, magic, max_dimension, None)
}

fn read_header(
    file: &mut fs::File,
    magic: &[u8; 8],
    max_dimension: u32,
    expected: Option<FrameSize>,
) -> Option<FrameSize> {
    let mut header = [0u8; 16];
    file.read_exact(&mut header).ok()?;
    if &header[..8] != magic {
        return None;
    }
    let size = FrameSize::new(
        u32::from_le_bytes([header[8], header[9], header[10], header[11]]),
        u32::from_le_bytes([header[12], header[13], header[14], header[15]]),
    );
    if size.is_empty()
        || size.width > max_dimension
        || size.height > max_dimension
        || expected.is_some_and(|expected| expected != size)
    {
        return None;
    }
    let byte_len = size.byte_len()?;
    let expected_len = 16u64.checked_add(u64::try_from(byte_len).ok()?)?;
    if file.metadata().ok()?.len() != expected_len {
        return None;
    }
    Some(size)
}

pub(crate) fn write_pixels(
    path: &Path,
    magic: &[u8; 8],
    size: FrameSize,
    pixels: &[u8],
) -> io::Result<()> {
    if size.is_empty() || size.byte_len() != Some(pixels.len()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid cached image size",
        ));
    }
    write_atomic(path, |file| {
        file.write_all(magic)?;
        file.write_all(&size.width.to_le_bytes())?;
        file.write_all(&size.height.to_le_bytes())?;
        file.write_all(pixels)
    })
}

#[cfg(test)]
mod tests;
