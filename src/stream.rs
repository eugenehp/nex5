//! Streaming reads without loading full variable payloads into memory.
#![allow(dead_code)] // public API used by callers; not all paths used in-crate

use crate::compat::Vec;
use crate::error::{map_io_err, NexError, Result};
use crate::format::{
    for_each_f64_chunk, for_each_timestamp_chunk, read_timestamps_range, VariableHeader,
};
use crate::io_ext::{IoResult, Read, Seek, SeekFrom};
use crate::read_options::ReadOptions;
use crate::validation::validate_variable_layout;
use crate::variables::NexFileVarType;

fn io_result_from_nex(result: Result<()>) -> IoResult<()> {
    result.map_err(|e| {
        #[cfg(feature = "std")]
        {
            std::io::Error::other(e.to_string())
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = e;
            crate::io_ext::Error::new(crate::io_ext::ErrorKind::Other)
        }
    })
}

/// Range of timestamps to read from a spike/event variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimestampRange {
    pub start: usize,
    pub count: usize,
}

impl TimestampRange {
    pub fn from_start_count(start: usize, count: usize) -> Self {
        Self { start, count }
    }

    pub fn first_n(n: usize) -> Self {
        Self { start: 0, count: n }
    }
}

/// Callback-driven timestamp reader for one variable payload.
pub struct TimestampStream<'a, R: Read + Seek + ?Sized> {
    reader: &'a mut R,
    header: &'a VariableHeader,
    freq: f64,
    chunk_size: usize,
    path: Option<&'a str>,
}

impl<'a, R: Read + Seek> TimestampStream<'a, R> {
    pub fn new(
        reader: &'a mut R,
        header: &'a VariableHeader,
        freq: f64,
        options: &ReadOptions,
    ) -> Self {
        Self {
            reader,
            header,
            freq,
            chunk_size: options.stream_chunk_size,
            path: None,
        }
    }

    pub fn with_path_label(mut self, path: Option<&'a str>) -> Self {
        self.path = path;
        self
    }

    fn seek_payload(&mut self) -> Result<()> {
        self.reader
            .seek(SeekFrom::Start(self.header.data_offset))
            .map_err(|e| map_io_err(self.path, e))?;
        Ok(())
    }

    /// Stream all timestamps through `callback` in chunks (no full `Vec` allocation).
    pub fn for_each(&mut self, mut callback: impl FnMut(&[f64]) -> Result<()>) -> Result<()> {
        self.seek_payload()?;
        let count = self.header.count as usize;
        for_each_timestamp_chunk(
            self.reader,
            count,
            self.header.ts_data_type,
            self.freq,
            self.chunk_size,
            |chunk| io_result_from_nex(callback(chunk)),
        )
        .map_err(|e| map_io_err(self.path, e))
    }

    /// Read only `range` timestamps into a new vector.
    pub fn read_range(&mut self, range: TimestampRange) -> Result<Vec<f64>> {
        self.seek_payload()?;
        read_timestamps_range(
            self.reader,
            self.header.count as usize,
            range.start,
            range.count,
            self.header.ts_data_type,
            self.freq,
        )
        .map_err(|e| map_io_err(self.path, e))
    }

    /// Collect the first `n` timestamps.
    pub fn read_first(&mut self, n: usize) -> Result<Vec<f64>> {
        let total = self.header.count as usize;
        self.read_range(TimestampRange {
            start: 0,
            count: n.min(total),
        })
    }
}

/// Stream continuous sample values (after fragment table) without full allocation.
pub fn stream_continuous_values<R: Read + Seek>(
    reader: &mut R,
    header: &VariableHeader,
    options: &ReadOptions,
    path: Option<&str>,
    file_size: u64,
    mut callback: impl FnMut(&[f64]) -> Result<()>,
) -> Result<()> {
    validate_variable_layout(header, file_size, options)?;
    if header.count == 0 {
        return Ok(());
    }
    reader
        .seek(SeekFrom::Start(header.data_offset))
        .map_err(|e| map_io_err(path, e))?;
    // Skip fragment timestamps + indexes
    let ts_bytes = (header.bytes_in_timestamp() as u64) * header.count;
    let index_bytes = 4 * header.count;
    reader
        .seek(SeekFrom::Current((ts_bytes + index_bytes) as i64))
        .map_err(|e| map_io_err(path, e))?;
    let (value_type, _, _) = header.cont_data_pars();
    let count = header.n_points_wave as usize;
    for_each_f64_chunk(
        reader,
        value_type,
        count,
        options.stream_chunk_size,
        |chunk| io_result_from_nex(callback(chunk)),
    )
    .map_err(|e| map_io_err(path, e))
}

/// Returns true if the variable type has a timestamp payload at its data offset.
pub fn variable_has_timestamps(var_type: i32) -> bool {
    matches!(
        NexFileVarType::from_i32(var_type),
        Some(
            NexFileVarType::Neuron
                | NexFileVarType::Event
                | NexFileVarType::Interval
                | NexFileVarType::Marker
                | NexFileVarType::Waveform
        )
    )
}

/// Sum spike/event counts without loading timestamps (uses header `count` only).
pub fn header_spike_count(header: &VariableHeader) -> u64 {
    header.count
}

/// Validate a timestamp range against a variable header.
pub fn validate_timestamp_range(header: &VariableHeader, range: TimestampRange) -> Result<()> {
    if range.start + range.count > header.count as usize {
        return Err(NexError::IncompleteRead);
    }
    Ok(())
}
