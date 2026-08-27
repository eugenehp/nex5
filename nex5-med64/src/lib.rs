//! Convert MED64 analysis output to [`nex5file::FileData`].
//!
//! Requires the sibling [`med64`](../../med64/rust/med64) crate and real `.modat` data.

use med64::{run_analysis, AnalysisOptions};
use nex5file::FileData;
use std::path::Path;

/// Options controlling MED64 → nex5 conversion.
#[derive(Debug, Clone)]
pub struct Med64ConvertOptions {
    pub analysis: AnalysisOptions,
    /// Timestamp frequency stored on the resulting nex5 file (Hz).
    pub timestamp_frequency_hz: f64,
    /// Limit blocks analyzed (useful for tests); `None` = all blocks.
    pub max_blocks: Option<usize>,
}

impl Default for Med64ConvertOptions {
    fn default() -> Self {
        Self {
            analysis: AnalysisOptions {
                max_blocks: Some(2),
                ..Default::default()
            },
            timestamp_frequency_hz: 40_000.0,
            max_blocks: Some(2),
        }
    }
}

/// Run spike detection on a MED64 recording and build a nex5 session.
pub fn modat_to_file_data(
    path: impl AsRef<Path>,
    options: &Med64ConvertOptions,
) -> med64::Result<FileData> {
    let path = path.as_ref();
    let mut analysis_opts = options.analysis.clone();
    if let Some(max) = options.max_blocks {
        analysis_opts.max_blocks = Some(max);
    }
    let report = run_analysis(path, &analysis_opts)?;

    let mut data = FileData::new(options.timestamp_frequency_hz, path.to_string_lossy()).map_err(
        |e| med64::Med64Error::InvalidWrite {
            message: e.to_string(),
        },
    )?;

    let by_channel = report.spikes.by_channel();
    for (channel, spikes) in by_channel {
        let name = format!("ch_{channel}");
        let timestamps: Vec<f64> = spikes
            .iter()
            .map(|s| s.within_session_time_ms / 1000.0)
            .collect();
        data.add_neuron(&name, timestamps, channel as i32, channel as i32, 0.0, 0.0)
            .map_err(|e| med64::Med64Error::InvalidWrite {
                message: e.to_string(),
            })?;
    }

    data.end_seconds = data
        .variables
        .iter()
        .map(|v| v.maximum_timestamp())
        .fold(0.0, f64::max);

    Ok(data)
}

/// Convert MED64 `.modat` to a `.nex5` file on disk.
pub fn modat_to_nex5(
    modat_path: impl AsRef<Path>,
    nex5_path: impl AsRef<Path>,
    options: &Med64ConvertOptions,
) -> med64::Result<()> {
    let data = modat_to_file_data(modat_path, options)?;
    nex5file::write_nex5_file(&data, nex5_path).map_err(|e| med64::Med64Error::InvalidWrite {
        message: e.to_string(),
    })
}
