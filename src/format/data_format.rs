use crate::compat::{round_f64, vec, Vec};
use crate::io_ext::{IoResult, Read, Write};

/// Elements per read/write chunk — balances stack buffer size and syscall count.
const IO_CHUNK_ELEMS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DataFormat {
    Int16 = 0,
    Uint16 = 1,
    Int32 = 2,
    Uint32 = 3,
    Int64 = 4,
    Uint64 = 5,
    Float32 = 6,
    Float64 = 7,
}

impl DataFormat {
    pub fn bytes_per_item(self) -> usize {
        match self {
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Int64 | Self::Uint64 | Self::Float64 => 8,
        }
    }

    pub fn read_f64_slice(self, reader: &mut impl Read, count: usize) -> IoResult<Vec<f64>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        match self {
            Self::Int32 => Ok(read_i32_vec(reader, count)?
                .into_iter()
                .map(f64::from)
                .collect()),
            Self::Int64 => Ok(read_i64_vec(reader, count)?
                .into_iter()
                .map(|v| v as f64)
                .collect()),
            Self::Uint32 => Ok(read_u32_vec(reader, count)?
                .into_iter()
                .map(f64::from)
                .collect()),
            Self::Float32 => Ok(read_f32_vec(reader, count)?
                .into_iter()
                .map(f64::from)
                .collect()),
            Self::Float64 => read_f64_vec(reader, count),
            Self::Int16 => Ok(read_i16_vec(reader, count)?
                .into_iter()
                .map(f64::from)
                .collect()),
            Self::Uint16 => Ok(read_u16_vec(reader, count)?
                .into_iter()
                .map(f64::from)
                .collect()),
            Self::Uint64 => Ok(read_u64_vec(reader, count)?
                .into_iter()
                .map(|v| v as f64)
                .collect()),
        }
    }

    pub fn read_f32_slice(self, reader: &mut impl Read, count: usize) -> IoResult<Vec<f32>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        match self {
            Self::Float32 => read_f32_vec(reader, count),
            Self::Int16 => Ok(read_i16_vec(reader, count)?
                .into_iter()
                .map(|v| v as f32)
                .collect()),
            _ => Ok(self
                .read_f64_slice(reader, count)?
                .into_iter()
                .map(|v| v as f32)
                .collect()),
        }
    }
}

pub fn read_i16_vec(reader: &mut impl Read, count: usize) -> IoResult<Vec<i16>> {
    read_le_vec(reader, count, 2, |b| {
        i16::from_le_bytes(b.try_into().unwrap())
    })
}

pub fn read_u16_vec(reader: &mut impl Read, count: usize) -> IoResult<Vec<u16>> {
    read_le_vec(reader, count, 2, |b| {
        u16::from_le_bytes(b.try_into().unwrap())
    })
}

pub fn read_i32_vec(reader: &mut impl Read, count: usize) -> IoResult<Vec<i32>> {
    read_le_vec(reader, count, 4, |b| {
        i32::from_le_bytes(b.try_into().unwrap())
    })
}

pub fn read_u32_vec(reader: &mut impl Read, count: usize) -> IoResult<Vec<u32>> {
    read_le_vec(reader, count, 4, |b| {
        u32::from_le_bytes(b.try_into().unwrap())
    })
}

pub fn read_i64_vec(reader: &mut impl Read, count: usize) -> IoResult<Vec<i64>> {
    read_le_vec(reader, count, 8, |b| {
        i64::from_le_bytes(b.try_into().unwrap())
    })
}

pub fn read_u64_vec(reader: &mut impl Read, count: usize) -> IoResult<Vec<u64>> {
    read_le_vec(reader, count, 8, |b| {
        u64::from_le_bytes(b.try_into().unwrap())
    })
}

pub fn read_f32_vec(reader: &mut impl Read, count: usize) -> IoResult<Vec<f32>> {
    read_le_vec(reader, count, 4, |b| {
        f32::from_le_bytes(b.try_into().unwrap())
    })
}

pub fn read_f64_vec(reader: &mut impl Read, count: usize) -> IoResult<Vec<f64>> {
    read_le_vec(reader, count, 8, |b| {
        f64::from_le_bytes(b.try_into().unwrap())
    })
}

/// Read timestamp ticks and convert to seconds in one allocation.
pub fn read_timestamps_as_f64(
    reader: &mut dyn Read,
    count: usize,
    ts_data_type: i32,
    freq: f64,
) -> IoResult<Vec<f64>> {
    let mut out = Vec::with_capacity(count);
    for_each_timestamp_chunk(reader, count, ts_data_type, freq, count.max(1), |chunk| {
        out.extend_from_slice(chunk);
        Ok(())
    })?;
    Ok(out)
}

/// Invoke `callback` with each chunk of timestamps (seconds) without one large allocation.
pub fn for_each_timestamp_chunk(
    reader: &mut dyn Read,
    count: usize,
    ts_data_type: i32,
    freq: f64,
    chunk_size: usize,
    mut callback: impl FnMut(&[f64]) -> IoResult<()>,
) -> IoResult<()> {
    if count == 0 {
        return Ok(());
    }
    let item_size = if ts_data_type == 0 { 4 } else { 8 };
    let chunk_size = chunk_size.max(1);
    let mut scratch_ts = vec![0.0f64; chunk_size];
    let mut buf = vec![0u8; chunk_size * item_size];
    let mut remaining = count;
    while remaining > 0 {
        let batch = remaining.min(chunk_size);
        let byte_len = batch * item_size;
        if buf.len() < byte_len {
            buf.resize(byte_len, 0);
        }
        reader.read_exact(&mut buf[..byte_len])?;
        if ts_data_type == 0 {
            for i in 0..batch {
                let ticks = i32::from_le_bytes(buf[i * 4..(i + 1) * 4].try_into().unwrap());
                scratch_ts[i] = f64::from(ticks) / freq;
            }
        } else {
            for i in 0..batch {
                let ticks = i64::from_le_bytes(buf[i * 8..(i + 1) * 8].try_into().unwrap());
                scratch_ts[i] = ticks as f64 / freq;
            }
        }
        callback(&scratch_ts[..batch])?;
        remaining -= batch;
    }
    Ok(())
}

/// Read a sub-range `[start, start + count)` of timestamps without loading the full series.
pub fn read_timestamps_range(
    reader: &mut dyn Read,
    total_count: usize,
    start: usize,
    count: usize,
    ts_data_type: i32,
    freq: f64,
) -> IoResult<Vec<f64>> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if start + count > total_count {
        #[cfg(feature = "std")]
        {
            return Err(std::io::Error::other("timestamp range out of bounds"));
        }
        #[cfg(not(feature = "std"))]
        {
            return Err(crate::io_ext::Error::new(crate::io_ext::ErrorKind::InvalidInput));
        }
    }
    let item_size = if ts_data_type == 0 { 4 } else { 8 };
    let skip_bytes = start * item_size;
    if skip_bytes > 0 {
        let mut discard = vec![0u8; skip_bytes.min(65536)];
        let mut left = skip_bytes;
        while left > 0 {
            let n = left.min(discard.len());
            reader.read_exact(&mut discard[..n])?;
            left -= n;
        }
    }
    read_timestamps_as_f64(reader, count, ts_data_type, freq)
}

/// Stream `f64` payload values in chunks (continuous, population vectors, etc.).
pub fn for_each_f64_chunk(
    reader: &mut dyn Read,
    format: DataFormat,
    count: usize,
    chunk_size: usize,
    mut callback: impl FnMut(&[f64]) -> IoResult<()>,
) -> IoResult<()> {
    if count == 0 {
        return Ok(());
    }
    let chunk_size = chunk_size.max(1);
    let mut scratch = vec![0.0f64; chunk_size];
    let item_size = format.bytes_per_item();
    let mut buf = vec![0u8; chunk_size * item_size];
    let mut remaining = count;
    while remaining > 0 {
        let batch = remaining.min(chunk_size);
        let byte_len = batch * item_size;
        if buf.len() < byte_len {
            buf.resize(byte_len, 0);
        }
        reader.read_exact(&mut buf[..byte_len])?;
        for i in 0..batch {
            scratch[i] = decode_f64(format, &buf[i * item_size..(i + 1) * item_size]);
        }
        callback(&scratch[..batch])?;
        remaining -= batch;
    }
    Ok(())
}

fn decode_f64(format: DataFormat, bytes: &[u8]) -> f64 {
    match format {
        DataFormat::Int16 => f64::from(i16::from_le_bytes(bytes.try_into().unwrap())),
        DataFormat::Uint16 => f64::from(u16::from_le_bytes(bytes.try_into().unwrap())),
        DataFormat::Int32 => f64::from(i32::from_le_bytes(bytes.try_into().unwrap())),
        DataFormat::Uint32 => f64::from(u32::from_le_bytes(bytes.try_into().unwrap())),
        DataFormat::Int64 => i64::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DataFormat::Uint64 => u64::from_le_bytes(bytes.try_into().unwrap()) as f64,
        DataFormat::Float32 => f64::from(f32::from_le_bytes(bytes.try_into().unwrap())),
        DataFormat::Float64 => f64::from_le_bytes(bytes.try_into().unwrap()),
    }
}

fn read_le_vec<T, F>(
    reader: &mut impl Read,
    count: usize,
    item_size: usize,
    decode: F,
) -> IoResult<Vec<T>>
where
    F: Fn(&[u8]) -> T,
{
    if count == 0 {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(count);
    let max_chunk_bytes = IO_CHUNK_ELEMS * item_size;
    let mut buf = vec![0u8; max_chunk_bytes.min(count * item_size)];
    let mut remaining = count;
    while remaining > 0 {
        let batch = remaining.min(IO_CHUNK_ELEMS);
        let byte_len = batch * item_size;
        if buf.len() < byte_len {
            buf.resize(byte_len, 0);
        }
        reader.read_exact(&mut buf[..byte_len])?;
        for i in 0..batch {
            out.push(decode(&buf[i * item_size..(i + 1) * item_size]));
        }
        remaining -= batch;
    }
    Ok(out)
}

#[allow(dead_code)]
pub fn write_i32_vec(writer: &mut impl Write, values: &[i32]) -> IoResult<()> {
    write_le_slice(writer, values, 4, |v| v.to_le_bytes())
}

#[allow(dead_code)]
pub fn write_i64_vec(writer: &mut impl Write, values: &[i64]) -> IoResult<()> {
    write_le_slice(writer, values, 8, |v| v.to_le_bytes())
}

pub fn write_u32_vec(writer: &mut impl Write, values: &[u32]) -> IoResult<()> {
    write_le_slice(writer, values, 4, |v| v.to_le_bytes())
}

#[allow(dead_code)]
pub fn write_i16_vec(writer: &mut impl Write, values: &[i16]) -> IoResult<()> {
    write_le_slice(writer, values, 2, |v| v.to_le_bytes())
}

#[allow(dead_code)]
pub fn write_f32_vec(writer: &mut impl Write, values: &[f32]) -> IoResult<()> {
    write_le_slice(writer, values, 4, |v| v.to_le_bytes())
}

pub fn write_f64_vec(writer: &mut impl Write, values: &[f64]) -> IoResult<()> {
    write_le_slice(writer, values, 8, |v| v.to_le_bytes())
}

pub fn write_timestamps_as_i32(
    writer: &mut impl Write,
    timestamps: &[f64],
    freq: f64,
) -> IoResult<()> {
    write_scaled_timestamps(
        writer,
        timestamps,
        freq,
        4,
        |v| v.to_le_bytes(),
        |v| v.to_le_bytes(),
    )
}

/// Write `f64` timestamps as i64 ticks without allocating the full tick vector.
pub fn write_timestamps_as_i64(
    writer: &mut impl Write,
    timestamps: &[f64],
    freq: f64,
) -> IoResult<()> {
    write_scaled_timestamps(
        writer,
        timestamps,
        freq,
        8,
        |v| v.to_le_bytes(),
        |v| v.to_le_bytes(),
    )
}

fn write_scaled_timestamps<W, F32, F64>(
    writer: &mut W,
    timestamps: &[f64],
    freq: f64,
    item_size: usize,
    to_le32: F32,
    to_le64: F64,
) -> IoResult<()>
where
    W: Write,
    F32: Fn(i32) -> [u8; 4],
    F64: Fn(i64) -> [u8; 8],
{
    if timestamps.is_empty() {
        return Ok(());
    }
    let mut buf = vec![0u8; IO_CHUNK_ELEMS * item_size];
    for chunk in timestamps.chunks(IO_CHUNK_ELEMS) {
        if item_size == 4 {
            for (i, &ts) in chunk.iter().enumerate() {
                let ticks = round_f64(ts * freq) as i32;
                buf[i * 4..(i + 1) * 4].copy_from_slice(&to_le32(ticks));
            }
        } else {
            for (i, &ts) in chunk.iter().enumerate() {
                let ticks = round_f64(ts * freq) as i64;
                buf[i * 8..(i + 1) * 8].copy_from_slice(&to_le64(ticks));
            }
        }
        writer.write_all(&buf[..chunk.len() * item_size])?;
    }
    Ok(())
}

fn write_le_slice<T, F, const N: usize>(
    writer: &mut impl Write,
    values: &[T],
    item_size: usize,
    to_le: F,
) -> IoResult<()>
where
    F: Fn(T) -> [u8; N],
    T: Copy,
{
    if values.is_empty() {
        return Ok(());
    }
    debug_assert_eq!(item_size, N);
    let mut buf = vec![0u8; IO_CHUNK_ELEMS * item_size];
    for chunk in values.chunks(IO_CHUNK_ELEMS) {
        for (i, &v) in chunk.iter().enumerate() {
            buf[i * item_size..(i + 1) * item_size].copy_from_slice(&to_le(v));
        }
        writer.write_all(&buf[..chunk.len() * item_size])?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn timestamps_to_i32(ticks: &[f64], freq: f64) -> Vec<i32> {
    ticks
        .iter()
        .map(|&ts| round_f64(ts * freq) as i32)
        .collect()
}

#[allow(dead_code)]
pub fn timestamps_to_i64(ticks: &[f64], freq: f64) -> Vec<i64> {
    ticks
        .iter()
        .map(|&ts| round_f64(ts * freq) as i64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "std")]
    use crate::io_ext::Cursor;

    #[test]
    #[cfg(feature = "std")]
    fn chunked_read_write_roundtrip_i32() {
        let values: Vec<i32> = (0..10_000).collect();
        let mut buf = Vec::new();
        write_i32_vec(&mut buf, &values).unwrap();
        let mut cursor = Cursor::new(buf);
        let back = read_i32_vec(&mut cursor, values.len()).unwrap();
        assert_eq!(back, values);
    }

    #[test]
    #[cfg(feature = "std")]
    fn write_timestamps_as_i32_matches_vec() {
        let ts = vec![0.001, 0.002, 0.003];
        let freq = 100_000.0;
        let mut bulk = Vec::new();
        write_timestamps_as_i32(&mut bulk, &ts, freq).unwrap();
        let mut vec_buf = Vec::new();
        write_i32_vec(&mut vec_buf, &timestamps_to_i32(&ts, freq)).unwrap();
        assert_eq!(bulk, vec_buf);
    }
}
