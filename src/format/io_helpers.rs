use crate::compat::{vec, String, ToString, Vec};
use crate::io_ext::{IoResult, Read, Write};

pub fn read_i32_le(reader: &mut impl Read) -> IoResult<i32> {
    Ok(i32::from_le_bytes(read_array::<4>(reader)?))
}

pub fn read_i64_le(reader: &mut impl Read) -> IoResult<i64> {
    Ok(i64::from_le_bytes(read_array::<8>(reader)?))
}

pub fn read_u64_le(reader: &mut impl Read) -> IoResult<u64> {
    Ok(u64::from_le_bytes(read_array::<8>(reader)?))
}

pub fn read_f64_le(reader: &mut impl Read) -> IoResult<f64> {
    Ok(f64::from_le_bytes(read_array::<8>(reader)?))
}

pub fn write_u32_le(writer: &mut impl Write, value: u32) -> IoResult<()> {
    writer.write_all(&value.to_le_bytes())
}

pub fn write_i32_le(writer: &mut impl Write, value: i32) -> IoResult<()> {
    writer.write_all(&value.to_le_bytes())
}

pub fn write_i64_le(writer: &mut impl Write, value: i64) -> IoResult<()> {
    writer.write_all(&value.to_le_bytes())
}

pub fn write_u64_le(writer: &mut impl Write, value: u64) -> IoResult<()> {
    writer.write_all(&value.to_le_bytes())
}

pub fn write_f64_le(writer: &mut impl Write, value: f64) -> IoResult<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_array<const N: usize>(reader: &mut impl Read) -> IoResult<[u8; N]> {
    let mut buf = [0u8; N];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_bytes(reader: &mut impl Read, count: usize) -> IoResult<Vec<u8>> {
    let mut buf = vec![0u8; count];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn write_padding(writer: &mut impl Write, size: usize) -> IoResult<()> {
    const ZERO: [u8; 256] = [0; 256];
    let mut remaining = size;
    while remaining > 0 {
        let n = remaining.min(ZERO.len());
        writer.write_all(&ZERO[..n])?;
        remaining -= n;
    }
    Ok(())
}

pub fn write_padded_string(writer: &mut impl Write, s: &str, size: usize) -> IoResult<()> {
    let mut buf = vec![0u8; size];
    let bytes = s.as_bytes();
    let len = bytes.len().min(size);
    buf[..len].copy_from_slice(&bytes[..len]);
    writer.write_all(&buf)
}

pub fn to_string(bytes: &[u8], discard_after_first_zero: bool) -> String {
    let slice = if discard_after_first_zero {
        bytes.split(|&b| b == 0).next().unwrap_or(bytes)
    } else {
        bytes
    };

    String::from_utf8_lossy(slice)
        .trim_matches('\0')
        .to_string()
}
