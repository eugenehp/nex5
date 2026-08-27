use crate::compat::{vec, String, ToString, Vec};
use crate::error::{map_io_err, NexError, Result};
use crate::file_data::FileData;
use crate::format::{
    read_bytes, read_f32_vec, read_f64_vec, read_i16_vec, read_timestamps_as_f64, read_u32_vec,
    to_string, DataFormat, FileHeader, VariableHeader,
};
use crate::io_ext::{Cursor, Read, Seek, SeekFrom};
use crate::read_options::ReadOptions;
use crate::validation::{validate_intervals, validate_variable_layout, verify_variable_header};
use crate::variables::{
    ContinuousVariable, MarkerFieldValue, MarkerVariable, Timestamps, Variable, WaveformVariable,
};

#[cfg(feature = "std")]
use crate::open_file::OpenNexFile;
#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::path::Path;

/// On-disk format selector for [`Reader::read_from_reader`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NexFormat {
    Nex,
    Nex5,
}

/// Byte source for [`Reader::read_from_reader`].
pub trait NexReadSeek: Read + Seek {}
impl<T: Read + Seek> NexReadSeek for T {}

/// Reads and parses NeuroExplorer `.nex` and `.nex5` files.
#[derive(Debug, Clone)]
pub struct Reader {
    options: ReadOptions,
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

impl Reader {
    pub fn new() -> Self {
        Self {
            options: ReadOptions::default(),
        }
    }

    pub fn with_options(options: ReadOptions) -> Self {
        Self { options }
    }

    pub fn options(&self) -> &ReadOptions {
        &self.options
    }

    /// Read from an in-memory byte slice (`no_std`-friendly).
    pub fn read_from_slice(&self, bytes: &[u8], format: NexFormat) -> Result<FileData> {
        let mut cursor = Cursor::new(bytes);
        self.read_from_reader(&mut cursor, format, bytes.len() as u64)
    }

    /// Read from any byte source implementing [`Read`] + [`Seek`].
    pub fn read_from_reader<R: NexReadSeek>(
        &self,
        reader: &mut R,
        format: NexFormat,
        file_size: u64,
    ) -> Result<FileData> {
        match format {
            NexFormat::Nex => self.read_nex_from(reader, None, file_size, None, false),
            NexFormat::Nex5 => self.read_nex5_from(reader, None, file_size, None, true),
        }
    }

    #[cfg(feature = "std")]
    /// Open a file and read all headers; payloads are loaded on demand via [`OpenNexFile::load_variables`].
    pub fn open_file<P: AsRef<Path>>(&self, path: P) -> Result<OpenNexFile> {
        let path = path.as_ref();
        let path_label = path.to_string_lossy();
        let mut file = File::open(path).map_err(|e| NexError::io(path_label.as_ref(), e))?;
        let file_size = file
            .metadata()
            .map_err(|e| NexError::io(path_label.as_ref(), e))?
            .len();
        let is_nex5 = !matches!(ext_lower(path).as_deref(), Some("nex"));
        let data = if is_nex5 {
            self.read_nex5_from(&mut file, Some(path_label.as_ref()), file_size, None, true)?
        } else {
            self.read_nex_from(&mut file, Some(path_label.as_ref()), file_size, None, false)?
        };
        Ok(OpenNexFile::from_parts(
            path.to_path_buf(),
            file,
            file_size,
            self.clone(),
            data,
            is_nex5,
        ))
    }

    #[cfg(feature = "std")]
    /// Open a file and read headers only (no payloads).
    pub fn open_headers_only<P: AsRef<Path>>(&self, path: P) -> Result<OpenNexFile> {
        let path = path.as_ref();
        let path_label = path.to_string_lossy();
        let mut file = File::open(path).map_err(|e| NexError::io(path_label.as_ref(), e))?;
        let file_size = file
            .metadata()
            .map_err(|e| NexError::io(path_label.as_ref(), e))?
            .len();
        let is_nex5 = !matches!(ext_lower(path).as_deref(), Some("nex"));
        let data = if is_nex5 {
            self.read_nex5_from(
                &mut file,
                Some(path_label.as_ref()),
                file_size,
                Some(Vec::new()),
                false,
            )?
        } else {
            self.read_nex_from(
                &mut file,
                Some(path_label.as_ref()),
                file_size,
                Some(Vec::new()),
                false,
            )?
        };
        Ok(OpenNexFile::from_parts(
            path.to_path_buf(),
            file,
            file_size,
            self.clone(),
            data,
            is_nex5,
        ))
    }

    #[cfg(feature = "std")]
    pub fn read_nex_file<P: AsRef<Path>>(&self, file_path: P) -> Result<FileData> {
        let path = file_path.as_ref();
        match ext_lower(path).as_deref() {
            Some("nex5") => self.read_nex5_file(path),
            _ => self.read_path_nex(path, None, false),
        }
    }

    #[cfg(feature = "std")]
    pub fn read_nex5_file<P: AsRef<Path>>(&self, file_path: P) -> Result<FileData> {
        let path = file_path.as_ref();
        match ext_lower(path).as_deref() {
            Some("nex") => self.read_path_nex(path, None, false),
            _ => self.read_path_nex5(path, None, true),
        }
    }

    #[cfg(feature = "std")]
    pub fn read_nex_headers_only<P: AsRef<Path>>(&self, file_path: P) -> Result<FileData> {
        let path = file_path.as_ref();
        match ext_lower(path).as_deref() {
            Some("nex5") => self.read_path_nex5(path, Some(Vec::new()), false),
            _ => self.read_path_nex(path, Some(Vec::new()), false),
        }
    }

    #[cfg(feature = "std")]
    pub fn read_nex5_headers_only<P: AsRef<Path>>(&self, file_path: P) -> Result<FileData> {
        self.read_nex_headers_only(file_path)
    }

    #[cfg(feature = "std")]
    pub fn read_nex_file_variables<P: AsRef<Path>>(
        &self,
        file_path: P,
        var_names: &[impl AsRef<str>],
    ) -> Result<FileData> {
        let path = file_path.as_ref();
        let names = collect_var_names(var_names);
        match ext_lower(path).as_deref() {
            Some("nex5") => self.read_path_nex5(path, Some(names), true),
            _ => self.read_path_nex(path, Some(names), false),
        }
    }

    #[cfg(feature = "std")]
    pub fn read_nex5_file_variables<P: AsRef<Path>>(
        &self,
        file_path: P,
        var_names: &[impl AsRef<str>],
    ) -> Result<FileData> {
        self.read_nex_file_variables(file_path, var_names)
    }

    #[cfg(feature = "std")]
    /// Load payload data for variables that were previously opened headers-only.
    pub fn load_variables<P: AsRef<Path>>(
        &self,
        file_path: P,
        data: &mut FileData,
        var_names: &[impl AsRef<str>],
    ) -> Result<()> {
        let path = file_path.as_ref();
        let path_label = path.to_string_lossy();
        let mut file = File::open(path).map_err(|e| NexError::io(path_label.as_ref(), e))?;
        let file_size = file
            .metadata()
            .map_err(|e| NexError::io(path_label.as_ref(), e))?
            .len();
        self.load_variables_on_handle(&mut file, file_size, path_label.as_ref(), data, var_names)
    }

    #[cfg(feature = "std")]
    pub(crate) fn load_variables_on_handle<R: Read + Seek>(
        &self,
        file: &mut R,
        file_size: u64,
        path: &str,
        data: &mut FileData,
        var_names: &[impl AsRef<str>],
    ) -> Result<()> {
        let mut indices = Vec::new();
        for name in var_names {
            let name = name.as_ref();
            let idx = data.index_of(name)?;
            if !data.variables[idx].is_loaded() {
                indices.push(idx);
            }
        }
        if self.options.sequential_io {
            indices.sort_by_key(|&idx| data.variables[idx].header().data_offset);
        }

        for idx in indices {
            let offset = data.variables[idx].header().data_offset;
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| NexError::io(path, e))?;
            self.read_one_variable_data(
                file,
                Some(path),
                file_size,
                &mut data.variables[idx],
                data.timestamp_frequency_hz,
            )?;
        }
        Ok(())
    }

    #[cfg(feature = "std")]
    fn read_path_nex(
        &self,
        path: &Path,
        payload_filter: Option<Vec<String>>,
        read_metadata: bool,
    ) -> Result<FileData> {
        let _ = read_metadata;
        let path_label = path.to_string_lossy();
        let mut file = File::open(path).map_err(|e| NexError::io(path_label.as_ref(), e))?;
        let file_size = file
            .metadata()
            .map_err(|e| NexError::io(path_label.as_ref(), e))?
            .len();
        self.read_nex_from(
            &mut file,
            Some(path_label.as_ref()),
            file_size,
            payload_filter,
            false,
        )
    }

    #[cfg(feature = "std")]
    fn read_path_nex5(
        &self,
        path: &Path,
        payload_filter: Option<Vec<String>>,
        read_metadata: bool,
    ) -> Result<FileData> {
        let path_label = path.to_string_lossy();
        let mut file = File::open(path).map_err(|e| NexError::io(path_label.as_ref(), e))?;
        let file_size = file
            .metadata()
            .map_err(|e| NexError::io(path_label.as_ref(), e))?
            .len();
        self.read_nex5_from(
            &mut file,
            Some(path_label.as_ref()),
            file_size,
            payload_filter,
            read_metadata,
        )
    }

    pub(crate) fn read_nex_from<R: Read + Seek>(
        &self,
        file: &mut R,
        path: Option<&str>,
        file_size: u64,
        payload_filter: Option<Vec<String>>,
        _read_metadata: bool,
    ) -> Result<FileData> {
        let file_header = FileHeader::read_from_nex(file)?;
        let mut data = FileData::from_header(&file_header);
        self.read_nex_variable_headers(path, file_size, file, file_header.num_vars, &mut data)?;
        if should_read_payload(payload_filter.as_deref()) {
            self.read_variable_data(file, path, file_size, &mut data, payload_filter.as_deref())?;
        }
        Ok(data)
    }

    pub(crate) fn read_nex5_from<R: Read + Seek>(
        &self,
        file: &mut R,
        path: Option<&str>,
        file_size: u64,
        payload_filter: Option<Vec<String>>,
        read_metadata: bool,
    ) -> Result<FileData> {
        let file_header = FileHeader::read_from_nex5(file)?;
        let mut data = FileData::from_header(&file_header);
        self.read_nex5_variable_headers(path, file_size, file, file_header.num_vars, &mut data)?;
        if should_read_payload(payload_filter.as_deref()) {
            self.read_variable_data(file, path, file_size, &mut data, payload_filter.as_deref())?;
        }
        if read_metadata {
            self.read_metadata(file, path, &file_header, &mut data)?;
        }
        Ok(data)
    }

    fn read_nex_variable_headers<R: Read>(
        &self,
        path: Option<&str>,
        file_size: u64,
        file: &mut R,
        num_vars: i32,
        data: &mut FileData,
    ) -> Result<()> {
        for _ in 0..num_vars {
            let vh = VariableHeader::read_from_nex(file).map_err(|e| map_io_err(path, e))?;
            verify_variable_header(&vh)?;
            validate_variable_layout(&vh, file_size, &self.options)?;
            data.variables.push(Variable::try_from_header(vh)?);
        }
        data.rebuild_index();
        Ok(())
    }

    fn read_nex5_variable_headers<R: Read>(
        &self,
        path: Option<&str>,
        file_size: u64,
        file: &mut R,
        num_vars: i32,
        data: &mut FileData,
    ) -> Result<()> {
        for _ in 0..num_vars {
            let vh = VariableHeader::read_from_nex5(file).map_err(|e| map_io_err(path, e))?;
            verify_variable_header(&vh)?;
            validate_variable_layout(&vh, file_size, &self.options)?;
            data.variables.push(Variable::try_from_header(vh)?);
        }
        data.rebuild_index();
        Ok(())
    }

    fn read_variable_data<R: Read + Seek>(
        &self,
        file: &mut R,
        path: Option<&str>,
        file_size: u64,
        data: &mut FileData,
        payload_filter: Option<&[String]>,
    ) -> Result<()> {
        #[cfg(feature = "parallel")]
        if self.options.parallel_decode {
            if let Some(path) = path {
                return self.read_variable_data_parallel(path, file_size, data, payload_filter);
            }
        }

        let freq = data.timestamp_frequency_hz;
        let mut order: Vec<usize> = (0..data.variables.len()).collect();
        if self.options.sequential_io {
            order.sort_by_key(|&i| data.variables[i].header().data_offset);
        }
        for i in order {
            if !should_load_payload(data.variables[i].name(), payload_filter) {
                continue;
            }
            let offset = data.variables[i].header().data_offset;
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| map_io_err(path, e))?;
            self.read_one_variable_data(file, path, file_size, &mut data.variables[i], freq)?;
        }
        Ok(())
    }

    #[cfg(feature = "parallel")]
    fn read_variable_data_parallel(
        &self,
        path: &str,
        file_size: u64,
        data: &mut FileData,
        payload_filter: Option<&[String]>,
    ) -> Result<()> {
        use rayon::prelude::*;

        let freq = data.timestamp_frequency_hz;
        let mut indices: Vec<usize> = (0..data.variables.len())
            .filter(|&i| should_load_payload(data.variables[i].name(), payload_filter))
            .collect();
        if self.options.sequential_io {
            indices.sort_by_key(|&i| data.variables[i].header().data_offset);
        }

        let loaded: Result<Vec<(usize, Variable)>> = indices
            .par_iter()
            .map(|&idx| {
                let mut file = File::open(path).map_err(|e| NexError::io(path, e))?;
                let mut var = data.variables[idx].clone();
                file.seek(SeekFrom::Start(var.header().data_offset))
                    .map_err(|e| NexError::io(path, e))?;
                self.read_one_variable_data(
                    &mut file,
                    Some(path),
                    file_size,
                    &mut var,
                    freq,
                )?;
                Ok((idx, var))
            })
            .collect();

        for (idx, var) in loaded? {
            data.variables[idx] = var;
        }
        Ok(())
    }

    fn read_one_variable_data<R: Read>(
        &self,
        file: &mut R,
        path: Option<&str>,
        file_size: u64,
        var: &mut Variable,
        freq: f64,
    ) -> Result<()> {
        validate_variable_layout(var.header(), file_size, &self.options)?;
        match var {
            Variable::Event(v) => {
                v.timestamps =
                    read_timestamps(file, path, &v.header, freq, self.options.compact_timestamps)?;
            }
            Variable::Neuron(v) => {
                v.timestamps =
                    read_timestamps(file, path, &v.header, freq, self.options.compact_timestamps)?;
            }
            Variable::Interval(v) => {
                v.interval_starts =
                    read_timestamps_f64(file, path, &v.header, freq, self.options.compact_timestamps)?;
                v.interval_ends =
                    read_timestamps_f64(file, path, &v.header, freq, self.options.compact_timestamps)?;
                validate_intervals(&v.interval_starts, &v.interval_ends)?;
            }
            Variable::Continuous(v) => self.read_continuous_data(file, path, v, freq)?,
            Variable::Waveform(v) => self.read_waveform_data(file, path, v, freq)?,
            Variable::Marker(v) => self.read_marker_data(file, path, v, freq)?,
            Variable::PopulationVector(v) => {
                let count = v.header.count as usize;
                v.weights = read_f64_vec(file, count).map_err(|e| map_io_err(path, e))?;
                if v.weights.len() != count {
                    return Err(NexError::IncompleteRead);
                }
            }
        }
        Ok(())
    }

    fn read_marker_data<R: Read>(
        &self,
        file: &mut R,
        path: Option<&str>,
        var: &mut MarkerVariable,
        freq: f64,
    ) -> Result<()> {
        if var.header.count == 0 {
            return Ok(());
        }
        var.timestamps =
            read_timestamps(file, path, &var.header, freq, self.options.compact_timestamps)?;
        for _ in 0..var.header.n_markers {
            let name = to_string(
                &read_bytes(file, 64).map_err(|e| map_io_err(path, e))?,
                true,
            );
            var.marker_field_names.push(name.trim().to_string());
            let markers = if var.header.marker_data_type == 0 {
                let length = var.header.marker_length as usize;
                let count = var.header.count as usize;
                let blob = read_bytes(file, count * length).map_err(|e| map_io_err(path, e))?;
                let mut values = Vec::with_capacity(count);
                for i in 0..count {
                    let start = i * length;
                    let s = to_string(&blob[start..start + length], true);
                    values.push(MarkerFieldValue::String(
                        s.trim_end_matches('\0').to_string(),
                    ));
                }
                values
            } else {
                read_u32_vec(file, var.header.count as usize)
                    .map_err(|e| map_io_err(path, e))?
                    .into_iter()
                    .map(MarkerFieldValue::Number)
                    .collect()
            };
            var.marker_fields.push(markers);
        }
        var.if_number_strings_store_as_numbers();
        Ok(())
    }

    fn read_waveform_data<R: Read>(
        &self,
        file: &mut R,
        path: Option<&str>,
        var: &mut WaveformVariable,
        freq: f64,
    ) -> Result<()> {
        if var.header.count == 0 {
            return Ok(());
        }
        var.timestamps =
            read_timestamps(file, path, &var.header, freq, self.options.compact_timestamps)?;
        let (cont_value_type, raw_to_mv, offset) = var.header.cont_data_pars();
        let count = (var.header.count * var.header.n_points_wave) as usize;
        var.waveform_values = match cont_value_type {
            DataFormat::Float32 => read_f32_vec(file, count).map_err(|e| map_io_err(path, e))?,
            DataFormat::Int16 => {
                let scale = raw_to_mv;
                read_i16_vec(file, count)
                    .map_err(|e| map_io_err(path, e))?
                    .into_iter()
                    .map(|v| (f64::from(v) * scale + offset) as f32)
                    .collect()
            }
            _ => {
                let mut wf =
                    read_and_scale_values(file, path, cont_value_type, count, raw_to_mv, false)?;
                if offset != 0.0 {
                    for v in &mut wf {
                        *v += offset;
                    }
                }
                wf.into_iter().map(|v| v as f32).collect()
            }
        };
        var.hash_cont_values();
        Ok(())
    }

    fn read_continuous_data<R: Read>(
        &self,
        file: &mut R,
        path: Option<&str>,
        var: &mut ContinuousVariable,
        freq: f64,
    ) -> Result<()> {
        if var.header.count == 0 {
            return Ok(());
        }
        var.fragment_timestamps =
            read_timestamps_f64(file, path, &var.header, freq, self.options.compact_timestamps)?;
        var.fragment_indexes =
            read_u32_vec(file, var.header.count as usize).map_err(|e| map_io_err(path, e))?;
        let (cont_value_type, raw_to_mv, offset) = var.header.cont_data_pars();
        var.continuous_values = match cont_value_type {
            DataFormat::Float32 => {
                let raw = read_f32_vec(file, var.header.n_points_wave as usize)
                    .map_err(|e| map_io_err(path, e))?;
                raw.into_iter().map(f64::from).collect()
            }
            _ => read_and_scale_values(
                file,
                path,
                cont_value_type,
                var.header.n_points_wave as usize,
                raw_to_mv,
                false,
            )?,
        };
        if offset != 0.0 {
            for v in &mut var.continuous_values {
                *v += offset;
            }
        }
        var.calculate_fragment_counts_from_indexes();
        var.hash_cont_values();
        Ok(())
    }

    fn read_metadata<R: Read + Seek>(
        &self,
        file: &mut R,
        path: Option<&str>,
        file_header: &FileHeader,
        data: &mut FileData,
    ) -> Result<()> {
        let meta_offset = file_header.meta_offset;
        if meta_offset == 0 {
            return Ok(());
        }
        let size = file
            .seek(SeekFrom::End(0))
            .map_err(|e| map_io_err(path, e))?;
        if meta_offset >= size {
            return Ok(());
        }
        file.seek(SeekFrom::Start(meta_offset))
            .map_err(|e| map_io_err(path, e))?;
        let mut buf = vec![0u8; (size - meta_offset) as usize];
        file.read_exact(&mut buf).map_err(|e| map_io_err(path, e))?;
        let meta_string = String::from_utf8_lossy(&buf)
            .trim_matches('\0')
            .trim()
            .to_string();

        match serde_json::from_str::<serde_json::Value>(&meta_string) {
            Ok(meta) => {
                data.metadata = meta;
                if let Some(all_var_meta) =
                    data.metadata.get("variables").and_then(|v| v.as_array())
                {
                    for var_meta in all_var_meta {
                        if let Some(name) = var_meta.get("name").and_then(|n| n.as_str()) {
                            if let Ok(idx) = data.index_of(name) {
                                *data.variables[idx].metadata_mut() = var_meta.clone();
                                if let Variable::Neuron(nr) = &mut data.variables[idx] {
                                    nr.assign_from_var_meta();
                                }
                            }
                        }
                    }
                }
            }
            Err(error) => {
                if self.options.ignore_metadata_errors {
                    return Ok(());
                }
                return Err(NexError::InvalidMetadata(error.to_string()));
            }
        }
        Ok(())
    }
}

fn should_read_payload(payload_filter: Option<&[String]>) -> bool {
    payload_filter.is_none() || payload_filter.is_some_and(|v| !v.is_empty())
}

fn should_load_payload(name: &str, payload_filter: Option<&[String]>) -> bool {
    match payload_filter {
        None => true,
        Some([]) => false,
        Some(names) => names.iter().any(|n| n == name),
    }
}

fn read_timestamps_f64(
    file: &mut impl Read,
    path: Option<&str>,
    header: &VariableHeader,
    freq: f64,
    compact: bool,
) -> Result<Vec<f64>> {
    read_timestamps(file, path, header, freq, compact).map(|ts| ts.as_f64_vec())
}

fn read_timestamps(
    file: &mut impl Read,
    path: Option<&str>,
    header: &VariableHeader,
    freq: f64,
    compact: bool,
) -> Result<Timestamps> {
    let count = header.count as usize;
    read_timestamps_as_f64(file, count, header.ts_data_type, freq)
        .map_err(|e| map_io_err(path, e))
        .and_then(|values| {
            if values.len() != count {
                Err(NexError::IncompleteRead)
            } else if compact {
                Ok(Timestamps::from_f64_compact(values))
            } else {
                Ok(Timestamps::from_f64(values))
            }
        })
}

fn read_and_scale_values(
    file: &mut impl Read,
    path: Option<&str>,
    value_type: DataFormat,
    count: usize,
    scale: f64,
    divide: bool,
) -> Result<Vec<f64>> {
    let mut values = value_type
        .read_f64_slice(file, count)
        .map_err(|e| map_io_err(path, e))?;
    if values.len() != count {
        return Err(NexError::IncompleteRead);
    }
    if scale != 1.0 {
        if divide {
            for v in &mut values {
                *v /= scale;
            }
        } else {
            for v in &mut values {
                *v *= scale;
            }
        }
    }
    Ok(values)
}

#[cfg(feature = "std")]
fn collect_var_names(names: &[impl AsRef<str>]) -> Vec<String> {
    names.iter().map(|n| n.as_ref().to_string()).collect()
}

#[cfg(feature = "std")]
fn ext_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

/// Convenience function — read using file extension to pick format.
#[cfg(feature = "std")]
pub fn read_nex_file<P: AsRef<Path>>(file_path: P) -> Result<FileData> {
    Reader::new().read_nex_file(file_path)
}

/// Convenience function — read as `.nex5` (or `.nex` if extension says so).
#[cfg(feature = "std")]
pub fn read_nex5_file<P: AsRef<Path>>(file_path: P) -> Result<FileData> {
    Reader::new().read_nex5_file(file_path)
}
