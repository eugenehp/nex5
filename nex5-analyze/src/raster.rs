//! Peri-event raster: spike times aligned to each event.
use crate::psth::PsthOptions;

/// Spikes aligned to one event (seconds relative to event time).
#[derive(Debug, Clone, PartialEq)]
pub struct TrialRaster {
    pub relative_spike_times: Vec<f64>,
}

/// Raster across all events.
#[derive(Debug, Clone, PartialEq)]
pub struct RasterResult {
    pub trials: Vec<TrialRaster>,
    pub n_events: usize,
}

/// Collect spike times relative to each event within the PSTH window.
pub fn peri_event_raster(
    spike_times: &[f64],
    event_times: &[f64],
    options: &PsthOptions,
) -> RasterResult {
    let mut sorted_spikes = spike_times.to_vec();
    sorted_spikes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

    let mut trials = Vec::with_capacity(event_times.len());
    for &event_t in event_times {
        let win_start = event_t + options.window_start;
        let win_end = event_t + options.window_end;
        let start_idx = sorted_spikes.partition_point(|&t| t < win_start);
        let mut relative_spike_times = Vec::new();
        for &spike_t in &sorted_spikes[start_idx..] {
            if spike_t > win_end {
                break;
            }
            relative_spike_times.push(spike_t - event_t);
        }
        trials.push(TrialRaster {
            relative_spike_times,
        });
    }

    RasterResult {
        n_events: event_times.len(),
        trials,
    }
}
