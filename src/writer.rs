use crate::compat::{round_f64, vec, ToString, Vec};
use crate::error::{map_io_err, NexError, Result};
use crate::file_data::FileData;
use crate::format::{
    write_f64_vec, write_padded_string, write_timestamps_as_i32, write_timestamps_as_i64,
    write_u32_le, write_u32_vec, write_u64_le, FileHeader, VariableHeader, NEX5_MAGIC, NEX_MAGIC,
};
use crate::io_ext::{Seek, SeekFrom, Write};
#[cfg(feature = "std")]
use crate::io_ext::Cursor;
use crate::reader::NexFormat;
use crate::variables::{
    ContinuousVariable, IntervalVariable, MarkerFieldValue, MarkerVariable, PopulationVector,
    Variable, WaveformVariable,
};
use crate::write_options::WriteOptions;
use crate::write_plan::{
    ensure_all_payloads_loaded, has_writable_payload, prepare_nex5_write_plan,
    prepare_nex_write_plan, FileWritePlan,
};

#[cfg(not(feature = "std"))]
use crate::io_ext::VecCursor;

#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::BufWriter;
#[cfg(feature = "std")]
use std::path::Path;

const META_OFFSET_IN_HEADER: u64 = 284;
const NEX5_I32_TIMESTAMP_LIMIT: f64 = 2_147_483_648.0;

/// Writes NeuroExplorer `.nex` and `.nex5` files.
#[derive(Debug, Clone)]
pub struct Writer {
    options: WriteOptions,
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl Writer {
    pub fn new() -> Self {
        Self {
            options: WriteOptions::default(),
        }
    }

    pub fn with_options(options: WriteOptions) -> Self {
        Self { options }
    }

    pub fn options(&self) -> &WriteOptions {
        &self.options
    }

    /// Serialize to an in-memory buffer (`no_std`-friendly).
    pub fn write_to_vec(&self, data: &FileData, format: NexFormat) -> Result<Vec<u8>> {
        #[cfg(feature = "std")]
        {
            let mut buf = Cursor::new(Vec::new());
            match format {
                NexFormat::Nex => self.write_nex_to(data, &mut buf)?,
                NexFormat::Nex5 => self.write_nex5_to(data, &mut buf)?,
            }
            Ok(buf.into_inner())
        }
        #[cfg(not(feature = "std"))]
        {
            let mut buf = VecCursor::new();
            match format {
                NexFormat::Nex => self.write_nex_to(data, &mut buf)?,
                NexFormat::Nex5 => self.write_nex5_to(data, &mut buf)?,
            }
            Ok(buf.into_inner())
        }
    }

    #[cfg(feature = "std")]
    pub fn write_nex_file<P: AsRef<Path>>(&self, data: &FileData, file_path: P) -> Result<()> {
        let path = file_path.as_ref();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            == Some("nex5".into())
        {
            return self.write_nex5_file(data, path);
        }
        self.write_nex_internal(data, path)
    }

    #[cfg(feature = "std")]
    pub fn write_nex5_file<P: AsRef<Path>>(&self, data: &FileData, file_path: P) -> Result<()> {
        let path = file_path.as_ref();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            == Some("nex".into())
        {
            return self.write_nex_internal(data, path);
        }
        self.write_nex5_internal(data, path)
    }

    pub fn write_nex_to<W: Write + Seek>(&self, data: &FileData, writer: &mut W) -> Result<()> {
        if !has_writable_payload(data) {
            return Err(NexError::NoVariableData);
        }
        ensure_all_payloads_loaded(data)?;
        let max_ts = data.maximum_timestamp();
        if round_f64(max_ts * data.timestamp_frequency_hz) > NEX5_I32_TIMESTAMP_LIMIT {
            return Err(NexError::TimestampExceeds32Bit);
        }

        let plan = prepare_nex_write_plan(data)?;
        let max_ts_ticks = round_f64(max_ts * data.timestamp_frequency_hz) as i64;
        let fh = prepare_nex_file_header(data, max_ts_ticks);
        fh.write_to_nex(writer).map_err(map_io_err_none)?;
        write_var_headers_nex(writer, &plan)?;
        write_variable_data(data, &plan, writer, data.timestamp_frequency_hz)?;
        writer.flush().map_err(map_io_err_none)?;
        Ok(())
    }

    pub fn write_nex5_to<W: Write + Seek>(&self, data: &FileData, writer: &mut W) -> Result<()> {
        if !has_writable_payload(data) {
            return Err(NexError::NoVariableData);
        }
        ensure_all_payloads_loaded(data)?;

        let plan = prepare_nex5_write_plan(data)?;
        let fh = prepare_nex5_file_header(data, plan.ts_as_64);
        fh.write_to_nex5(writer).map_err(map_io_err_none)?;
        write_var_headers_nex5(writer, &plan)?;
        write_variable_data(data, &plan, writer, data.timestamp_frequency_hz)?;
        if self.options.embed_metadata {
            write_metadata(data, &self.options, writer)?;
        }
        writer.flush().map_err(map_io_err_none)?;
        Ok(())
    }

    #[cfg(feature = "std")]
    fn write_nex_internal(&self, data: &FileData, path: &Path) -> Result<()> {
        let path_label = path.to_string_lossy();
        let file = File::create(path).map_err(|e| NexError::io(path_label.as_ref(), e))?;
        let mut file = BufWriter::with_capacity(self.options.buffer_bytes, file);
        self.write_nex_to(data, &mut file)
    }

    #[cfg(feature = "std")]
    fn write_nex5_internal(&self, data: &FileData, path: &Path) -> Result<()> {
        let path_label = path.to_string_lossy();
        let file = File::create(path).map_err(|e| NexError::io(path_label.as_ref(), e))?;
        let mut file = BufWriter::with_capacity(self.options.buffer_bytes, file);
        self.write_nex5_to(data, &mut file)
    }
}

fn map_io_err_none(source: impl core::fmt::Display) -> NexError {
    map_io_err(None, source)
}

fn write_var_headers_nex(file: &mut impl Write, plan: &FileWritePlan) -> Result<()> {
    for entry in &plan.variables {
        entry
            .write_header
            .write_to_nex(file)
            .map_err(map_io_err_none)?;
    }
    Ok(())
}

fn write_var_headers_nex5(file: &mut impl Write, plan: &FileWritePlan) -> Result<()> {
    for entry in &plan.variables {
        entry
            .write_header
            .write_to_nex5(file)
            .map_err(map_io_err_none)?;
    }
    Ok(())
}

fn write_variable_data(
    data: &FileData,
    plan: &FileWritePlan,
    file: &mut impl Write,
    freq: f64,
) -> Result<()> {
    for (var, entry) in data.variables.iter().zip(&plan.variables) {
        let hw = &entry.write_header;
        match var {
            Variable::Event(v) => write_timestamps(file, hw, &v.timestamps.as_f64_vec(), freq)?,
            Variable::Neuron(v) => write_timestamps(file, hw, &v.timestamps.as_f64_vec(), freq)?,
            Variable::Interval(v) => write_intervals(file, hw, v, freq)?,
            Variable::Marker(v) => write_markers(file, hw, v, freq)?,
            Variable::Continuous(v) => write_continuous(file, hw, v, freq)?,
            Variable::Waveform(v) => write_waveform(file, hw, v, freq)?,
            Variable::PopulationVector(v) => write_pop_vector(file, v)?,
        }
    }
    Ok(())
}

fn write_timestamps(
    file: &mut impl Write,
    hw: &VariableHeader,
    timestamps: &[f64],
    freq: f64,
) -> Result<()> {
    if hw.bytes_in_timestamp() == 4 {
        write_timestamps_as_i32(file, timestamps, freq).map_err(map_io_err_none)
    } else {
        write_timestamps_as_i64(file, timestamps, freq).map_err(map_io_err_none)
    }
}

fn write_intervals(
    file: &mut impl Write,
    hw: &VariableHeader,
    var: &IntervalVariable,
    freq: f64,
) -> Result<()> {
    write_timestamps(file, hw, &var.interval_starts, freq)?;
    write_timestamps(file, hw, &var.interval_ends, freq)
}

fn write_pop_vector(file: &mut impl Write, var: &PopulationVector) -> Result<()> {
    write_f64_vec(file, &var.weights).map_err(map_io_err_none)
}

fn write_markers(
    file: &mut impl Write,
    hw: &VariableHeader,
    var: &MarkerVariable,
    freq: f64,
) -> Result<()> {
    write_timestamps(file, hw, &var.timestamps.as_f64_vec(), freq)?;
    for (i, name) in var.marker_field_names.iter().enumerate() {
        write_padded_string(file, name, 64).map_err(map_io_err_none)?;
        if hw.marker_data_type == 1 {
            for value in &var.marker_fields[i] {
                let n = match value {
                    MarkerFieldValue::Number(n) => *n,
                    MarkerFieldValue::String(s) => {
                        return Err(NexError::MarkerStringInNumericMode(s.clone()));
                    }
                };
                write_u32_le(file, n).map_err(map_io_err_none)?;
            }
        } else {
            for value in &var.marker_fields[i] {
                let mut sv = match value {
                    MarkerFieldValue::Number(n) => crate::compat::format!("{n:05}\0"),
                    MarkerFieldValue::String(s) => {
                        let mut s = s.clone();
                        while s.len() < hw.marker_length as usize {
                            s.push('\0');
                        }
                        s
                    }
                };
                if sv.len() < hw.marker_length as usize {
                    while sv.len() < hw.marker_length as usize {
                        sv.push('\0');
                    }
                }
                file.write_all(sv.as_bytes()).map_err(map_io_err_none)?;
            }
        }
    }
    Ok(())
}

fn write_continuous(
    file: &mut impl Write,
    hw: &VariableHeader,
    var: &ContinuousVariable,
    freq: f64,
) -> Result<()> {
    write_timestamps(file, hw, &var.fragment_timestamps, freq)?;
    write_u32_vec(file, &var.fragment_indexes).map_err(map_io_err_none)?;
    if hw.cont_data_type == 0 {
        write_i16_from_f64(file, &var.continuous_values, hw.ad_to_mv)
    } else {
        write_f32_from_f64(file, &var.continuous_values)
    }
}

fn write_waveform(
    file: &mut impl Write,
    hw: &VariableHeader,
    var: &WaveformVariable,
    freq: f64,
) -> Result<()> {
    write_timestamps(file, hw, &var.timestamps.as_f64_vec(), freq)?;
    if hw.cont_data_type == 0 {
        write_i16_from_f32_flat(file, &var.waveform_values, hw.ad_to_mv)
    } else {
        write_f32_flat(file, &var.waveform_values)
    }
}

fn write_i16_from_f64(file: &mut impl Write, values: &[f64], ad_to_mv: f64) -> Result<()> {
    const CHUNK: usize = 4096;
    let mut buf = vec![0u8; CHUNK * 2];
    for chunk in values.chunks(CHUNK) {
        for (i, &v) in chunk.iter().enumerate() {
            let raw = round_f64(v / ad_to_mv) as i16;
            buf[i * 2..(i + 1) * 2].copy_from_slice(&raw.to_le_bytes());
        }
        file.write_all(&buf[..chunk.len() * 2])
            .map_err(map_io_err_none)?;
    }
    Ok(())
}

fn write_f32_from_f64(file: &mut impl Write, values: &[f64]) -> Result<()> {
    const CHUNK: usize = 4096;
    let mut buf = vec![0u8; CHUNK * 4];
    for chunk in values.chunks(CHUNK) {
        for (i, &v) in chunk.iter().enumerate() {
            buf[i * 4..(i + 1) * 4].copy_from_slice(&(v as f32).to_le_bytes());
        }
        file.write_all(&buf[..chunk.len() * 4])
            .map_err(map_io_err_none)?;
    }
    Ok(())
}

fn write_f32_flat(file: &mut impl Write, values: &[f32]) -> Result<()> {
    const CHUNK: usize = 4096;
    let mut buf = vec![0u8; CHUNK * 4];
    for chunk in values.chunks(CHUNK) {
        for (i, &v) in chunk.iter().enumerate() {
            buf[i * 4..(i + 1) * 4].copy_from_slice(&v.to_le_bytes());
        }
        file.write_all(&buf[..chunk.len() * 4])
            .map_err(map_io_err_none)?;
    }
    Ok(())
}

fn write_i16_from_f32_flat(file: &mut impl Write, values: &[f32], ad_to_mv: f64) -> Result<()> {
    const CHUNK: usize = 4096;
    let mut buf = vec![0u8; CHUNK * 2];
    for chunk in values.chunks(CHUNK) {
        for (i, &v) in chunk.iter().enumerate() {
            let raw = round_f64(f64::from(v) / ad_to_mv) as i16;
            buf[i * 2..(i + 1) * 2].copy_from_slice(&raw.to_le_bytes());
        }
        file.write_all(&buf[..chunk.len() * 2])
            .map_err(map_io_err_none)?;
    }
    Ok(())
}

fn write_metadata(
    data: &FileData,
    options: &WriteOptions,
    file: &mut (impl Write + Seek),
) -> Result<()> {
    let mut meta = serde_json::json!({
        "file": {
            "writerSoftware": {
                "name": options.writer_name,
                "version": options.writer_version,
            }
        },
        "variables": []
    });

    let variables = meta
        .get_mut("variables")
        .and_then(|v| v.as_array_mut())
        .expect("variables array");

    for var in &data.variables {
        let mut var_meta = serde_json::json!({ "name": var.name() });
        if matches!(var, Variable::Neuron(_) | Variable::Waveform(_)) {
            let h = var.header();
            var_meta["unitNumber"] = serde_json::json!(h.unit);
            var_meta["probe"] = serde_json::json!({
                "wireNumber": h.wire,
                "position": { "x": h.x_pos, "y": h.y_pos }
            });
        }
        variables.push(var_meta);
    }

    let meta_string =
        serde_json::to_vec(&meta).map_err(|e| NexError::InvalidMetadata(e.to_string()))?;
    let pos = file.stream_position().map_err(map_io_err_none)?;
    file.write_all(&meta_string).map_err(map_io_err_none)?;
    file.seek(SeekFrom::Start(META_OFFSET_IN_HEADER))
        .map_err(map_io_err_none)?;
    write_u64_le(file, pos).map_err(map_io_err_none)?;
    Ok(())
}

fn prepare_nex_file_header(data: &FileData, max_ts_ticks: i64) -> FileHeader {
    FileHeader {
        magic_number: NEX_MAGIC,
        nex_file_version: 106,
        comment: data.comment.clone(),
        frequency: data.timestamp_frequency_hz,
        beg_ticks: round_f64(data.beg_seconds * data.timestamp_frequency_hz) as i64,
        end_ticks: max_ts_ticks,
        num_vars: data.variables.len() as i32,
        ..Default::default()
    }
}

fn prepare_nex5_file_header(data: &FileData, ts_as_64: i32) -> FileHeader {
    let max_ts = data.maximum_timestamp();
    let max_ts_ticks = round_f64(max_ts * data.timestamp_frequency_hz) as i64;
    let version = if ts_as_64 == 1 { 502 } else { 501 };
    FileHeader {
        magic_number: NEX5_MAGIC,
        nex_file_version: version,
        comment: data.comment.clone(),
        frequency: data.timestamp_frequency_hz,
        beg_ticks: round_f64(data.beg_seconds * data.timestamp_frequency_hz) as i64,
        end_ticks: max_ts_ticks,
        num_vars: data.variables.len() as i32,
        ..Default::default()
    }
}

/// Convenience function — write using file extension to pick format.
#[cfg(feature = "std")]
pub fn write_nex_file<P: AsRef<Path>>(data: &FileData, file_path: P) -> Result<()> {
    Writer::new().write_nex_file(data, file_path)
}

/// Convenience function — write as `.nex5` (or `.nex` if extension says so).
#[cfg(feature = "std")]
pub fn write_nex5_file<P: AsRef<Path>>(data: &FileData, file_path: P) -> Result<()> {
    Writer::new().write_nex5_file(data, file_path)
}

/// Convenience function with custom write options.
#[cfg(feature = "std")]
pub fn write_nex5_file_with_options<P: AsRef<Path>>(
    data: &FileData,
    file_path: P,
    options: WriteOptions,
) -> Result<()> {
    Writer::with_options(options).write_nex5_file(data, file_path)
}
