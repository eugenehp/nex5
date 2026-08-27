//! Export in-memory data to common interchange formats (I/O only, no analysis).

use crate::compat::{format, String, Vec};
use crate::error::{NexError, Result};
use crate::file_data::FileData;
use crate::io_ext::Write;

/// Export format for spike timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpikeExportFormat {
    /// One timestamp per line (seconds).
    Csv,
    /// NeuroExplorer-compatible text (one column).
    Text,
}

/// Options for spike export.
#[derive(Debug, Clone)]
pub struct SpikeExportOptions {
    pub format: SpikeExportFormat,
    pub include_header: bool,
}

impl Default for SpikeExportOptions {
    fn default() -> Self {
        Self {
            format: SpikeExportFormat::Csv,
            include_header: true,
        }
    }
}

impl SpikeExportOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn format(mut self, format: SpikeExportFormat) -> Self {
        self.format = format;
        self
    }

    pub fn include_header(mut self, include: bool) -> Self {
        self.include_header = include;
        self
    }
}

/// Write spike timestamps for `var_name` to any `Write` target.
pub fn export_spikes<W: Write>(
    data: &FileData,
    var_name: &str,
    writer: &mut W,
    options: &SpikeExportOptions,
) -> Result<()> {
    let var = data.get_variable(var_name)?;
    let timestamps = var.timestamps().map_err(|_| {
        NexError::WrongVariableType(var_name.to_string(), "neuron, event, marker, or waveform")
    })?;
    if options.include_header {
        let header = match options.format {
            SpikeExportFormat::Csv => "timestamp_sec\n",
            SpikeExportFormat::Text => "# timestamp_sec\n",
        };
        writer
            .write_all(header.as_bytes())
            .map_err(|e| NexError::io(var_name, e))?;
    }
    for ts in timestamps {
        let line = format!("{ts}\n");
        writer
            .write_all(line.as_bytes())
            .map_err(|e| NexError::io(var_name, e))?;
    }
    Ok(())
}

#[cfg(feature = "std")]
/// Write spike timestamps to a file.
pub fn export_spikes_to_file(
    data: &FileData,
    var_name: &str,
    path: impl AsRef<std::path::Path>,
    options: &SpikeExportOptions,
) -> Result<()> {
    use std::fs::File;
    use std::io::BufWriter;
    let path = path.as_ref();
    let file = File::create(path).map_err(|e| NexError::io(path.to_string_lossy(), e))?;
    let mut writer = BufWriter::new(file);
    export_spikes(data, var_name, &mut writer, options)?;
    writer
        .flush()
        .map_err(|e| NexError::io(path.to_string_lossy(), e))
}

/// List neuron and event variable names suitable for spike export.
pub fn spike_export_names(data: &FileData) -> Vec<String> {
    let mut names = data.neuron_names();
    names.extend(data.event_names());
    names.sort();
    names.dedup();
    names
}
