use nex5file::FileData;

use crate::filter::{filter_file_neuron, TimeRange};
use crate::psth::{psth, PsthOptions, PsthResult};
use crate::raster::{peri_event_raster, RasterResult};

/// Convenience bundle for a neuron/event PSTH analysis on a file.
#[derive(Debug, Clone, Default)]
pub struct FileAnalysisOptions {
    pub psth: PsthOptions,
    pub spike_range: Option<TimeRange>,
}

impl FileAnalysisOptions {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub struct FileAnalysisResult {
    pub psth: PsthResult,
    pub raster: RasterResult,
    pub spike_times: Vec<f64>,
    pub event_times: Vec<f64>,
}

/// Run PSTH + raster for one neuron aligned to one event variable.
pub fn analyze_file(
    data: &FileData,
    neuron: &str,
    event: &str,
    options: &FileAnalysisOptions,
) -> crate::Result<FileAnalysisResult> {
    let mut spike_times = if let Some(range) = options.spike_range {
        filter_file_neuron(data, neuron, range)?
    } else {
        data.neuron(neuron)?.timestamps.as_f64_vec()
    };
    spike_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let event_times = data.event(event)?.timestamps.as_f64_vec();
    let psth_result = psth(&spike_times, &event_times, &options.psth);
    let raster = peri_event_raster(&spike_times, &event_times, &options.psth);
    Ok(FileAnalysisResult {
        psth: psth_result,
        raster,
        spike_times,
        event_times,
    })
}
