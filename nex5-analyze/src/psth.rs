/// Peri-stimulus histogram configuration.
#[derive(Debug, Clone)]
pub struct PsthOptions {
    /// Window start relative to each event time (seconds).
    pub window_start: f64,
    /// Window end relative to each event time (seconds).
    pub window_end: f64,
    /// Bin width in seconds.
    pub bin_width: f64,
}

impl Default for PsthOptions {
    fn default() -> Self {
        Self {
            window_start: -0.5,
            window_end: 1.0,
            bin_width: 0.01,
        }
    }
}

/// Peri-stimulus histogram result.
#[derive(Debug, Clone, PartialEq)]
pub struct PsthResult {
    /// Left edges of each bin (seconds relative to event).
    pub bin_edges: Vec<f64>,
    /// Spike counts per bin (summed across all events).
    pub counts: Vec<u64>,
    /// Firing rate per bin (Hz), normalized by event count and bin width.
    pub rate_hz: Vec<f64>,
    /// Number of events used.
    pub n_events: usize,
}

/// Build a peri-stimulus time histogram from spike times and event times.
///
/// Spike and event times must be in the same time base (seconds).
pub fn psth(spike_times: &[f64], event_times: &[f64], options: &PsthOptions) -> PsthResult {
    assert!(options.bin_width > 0.0, "bin_width must be positive");
    assert!(
        options.window_end > options.window_start,
        "window_end must exceed window_start"
    );

    let n_bins = ((options.window_end - options.window_start) / options.bin_width).ceil() as usize;
    let mut counts = vec![0u64; n_bins.max(1)];
    let mut bin_edges: Vec<f64> = (0..=n_bins)
        .map(|i| options.window_start + i as f64 * options.bin_width)
        .collect();
    if bin_edges.len() > counts.len() {
        bin_edges.pop();
    }

    if event_times.is_empty() || spike_times.is_empty() {
        let rate_hz = counts.iter().map(|_| 0.0).collect();
        return PsthResult {
            bin_edges,
            counts,
            rate_hz,
            n_events: event_times.len(),
        };
    }

    let mut sorted_spikes = spike_times.to_vec();
    sorted_spikes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

    for &event_t in event_times {
        let win_start = event_t + options.window_start;
        let win_end = event_t + options.window_end;

        let start_idx = sorted_spikes.partition_point(|&t| t < win_start);
        for &spike_t in &sorted_spikes[start_idx..] {
            if spike_t > win_end {
                break;
            }
            let rel = spike_t - event_t;
            if rel < options.window_start || rel >= options.window_end {
                continue;
            }
            let bin = ((rel - options.window_start) / options.bin_width) as usize;
            if bin < counts.len() {
                counts[bin] += 1;
            }
        }
    }

    let n_events = event_times.len();
    let rate_hz = counts
        .iter()
        .map(|&c| c as f64 / (n_events as f64 * options.bin_width))
        .collect();

    PsthResult {
        bin_edges,
        counts,
        rate_hz,
        n_events,
    }
}
