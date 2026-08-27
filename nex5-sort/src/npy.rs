//! Minimal NumPy `.npy` reader/writer for Kilosort/Phy exports.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::{Result, SortError};

/// Build a v1.0 `.npy` header block padded to a 16-byte boundary.
pub(crate) fn format_npy_header(dict: &str) -> String {
    let mut header = format!("{dict}\n");
    let prefix_len = 10usize; // magic(6) + version(2) + header_len(2)
    let pad = (16 - ((prefix_len + header.len()) % 16)) % 16;
    header.push_str(&" ".repeat(pad));
    header
}

/// Read a 1-D `int32` vector from a `.npy` file.
pub fn read_i32_1d(path: impl AsRef<Path>) -> Result<Vec<i32>> {
    let (descr, data) = read_npy_payload(path.as_ref())?;
    if !descr.contains("<i4") {
        return Err(SortError::Npy(format!(
            "{}: expected <i4, got {descr}",
            path.as_ref().display()
        )));
    }
    Ok(decode_i32_le(&data))
}

/// Read spike sample indices from Kilosort/Phy `spike_times.npy` (supports `<f8`, `<f4`, `<i8`, `<i4>`).
pub fn read_spike_times_npy(path: impl AsRef<Path>) -> Result<Vec<f64>> {
    let path = path.as_ref();
    let (descr, data) = read_npy_payload(path)?;
    if descr.contains("<f8") {
        Ok(decode_f64_le(&data))
    } else if descr.contains("<f4") {
        Ok(decode_f32_as_f64(&data))
    } else if descr.contains("<i8") {
        Ok(decode_i64_as_f64(&data))
    } else if descr.contains("<i4") {
        Ok(decode_i32_as_f64(&data))
    } else {
        Err(SortError::Npy(format!(
            "{}: unsupported spike_times dtype in {descr}",
            path.display()
        )))
    }
}

fn read_npy_payload(path: &Path) -> Result<(String, Vec<u8>)> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 6];
    file.read_exact(&mut magic)?;
    if &magic != b"\x93NUMPY" {
        return Err(SortError::Npy(format!("{}: not npy", path.display())));
    }
    let mut ver = [0u8; 2];
    file.read_exact(&mut ver)?;
    let header_len = if ver == [1, 0] {
        let mut len_buf = [0u8; 2];
        file.read_exact(&mut len_buf)?;
        u16::from_le_bytes(len_buf) as usize
    } else {
        let mut len_buf = [0u8; 4];
        file.read_exact(&mut len_buf)?;
        u32::from_le_bytes(len_buf) as usize
    };
    let mut header = vec![0u8; header_len];
    file.read_exact(&mut header)?;
    let header_str = String::from_utf8_lossy(&header).into_owned();
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    Ok((header_str, data))
}

fn decode_f64_le(data: &[u8]) -> Vec<f64> {
    data.chunks_exact(8)
        .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

fn decode_f32_as_f64(data: &[u8]) -> Vec<f64> {
    data.chunks_exact(4)
        .map(|chunk| f64::from(f32::from_le_bytes(chunk.try_into().unwrap())))
        .collect()
}

fn decode_i32_as_f64(data: &[u8]) -> Vec<f64> {
    data.chunks_exact(4)
        .map(|chunk| f64::from(i32::from_le_bytes(chunk.try_into().unwrap())))
        .collect()
}

fn decode_i64_as_f64(data: &[u8]) -> Vec<f64> {
    data.chunks_exact(8)
        .map(|chunk| i64::from_le_bytes(chunk.try_into().unwrap()) as f64)
        .collect()
}

fn decode_i32_le(data: &[u8]) -> Vec<i32> {
    data.chunks_exact(4)
        .map(|chunk| i32::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_npy(path: &Path, descr: &str, payload: &[u8]) {
        let header = format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': (2,), }}");
        let padded = format_npy_header(&header);
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(b"\x93NUMPY\x01\x00").unwrap();
        f.write_all(&(padded.len() as u16).to_le_bytes()).unwrap();
        f.write_all(padded.as_bytes()).unwrap();
        f.write_all(payload).unwrap();
    }

    #[test]
    fn reads_simple_f64_npy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.npy");
        let mut payload = Vec::new();
        payload.extend_from_slice(&1.0f64.to_le_bytes());
        payload.extend_from_slice(&2.0f64.to_le_bytes());
        write_npy(&path, "<f8", &payload);
        assert_eq!(read_spike_times_npy(&path).unwrap(), vec![1.0, 2.0]);
    }

    #[test]
    fn reads_i64_spike_times() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("spike_times.npy");
        let mut payload = Vec::new();
        payload.extend_from_slice(&100i64.to_le_bytes());
        payload.extend_from_slice(&200i64.to_le_bytes());
        write_npy(&path, "<i8", &payload);
        assert_eq!(read_spike_times_npy(&path).unwrap(), vec![100.0, 200.0]);
    }
}
