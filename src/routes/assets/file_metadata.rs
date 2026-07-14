use crate::error::{AppError, AppResult};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

const QUICK_HASH_SAMPLE_BYTES: usize = 256 * 1024;
const QUICK_HASH_VERSION_MARKER: &[u8] = b"madlibrary-quick-hash-v2-sampled-256k";

pub(super) fn compute_asset_quick_hash(path: &Path, size_bytes: u64) -> AppResult<String> {
    let mut file = fs::File::open(path)
        .map_err(|error| AppError::BadRequest(format!("could not hash asset: {error}")))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(QUICK_HASH_VERSION_MARKER);
    hasher.update(&size_bytes.to_le_bytes());

    let sample_len = QUICK_HASH_SAMPLE_BYTES.min(size_bytes as usize);
    let middle_offset = size_bytes.saturating_sub(sample_len as u64) / 2;
    let tail_offset = size_bytes.saturating_sub(sample_len as u64);
    let mut previous_offset = None;
    for offset in [0, middle_offset, tail_offset] {
        if previous_offset == Some(offset) {
            continue;
        }
        previous_offset = Some(offset);
        hasher.update(&offset.to_le_bytes());
        update_hash_from_file_range(&mut file, &mut hasher, offset, sample_len)?;
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn update_hash_from_file_range(
    file: &mut fs::File,
    hasher: &mut blake3::Hasher,
    offset: u64,
    length: usize,
) -> AppResult<()> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| AppError::BadRequest(format!("could not hash asset: {error}")))?;
    let mut remaining = length;
    let mut buffer = vec![0_u8; 64 * 1024];

    while remaining > 0 {
        let read_len = remaining.min(buffer.len());
        let bytes_read = file
            .read(&mut buffer[..read_len])
            .map_err(|error| AppError::BadRequest(format!("could not hash asset: {error}")))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
        remaining -= bytes_read;
    }
    Ok(())
}
