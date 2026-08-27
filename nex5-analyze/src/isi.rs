/// Inter-spike interval histogram.
#[derive(Debug, Clone, PartialEq)]
pub struct IsiHistogram {
    pub bin_edges: Vec<f64>,
    pub counts: Vec<u64>,
}

/// Consecutive inter-spike intervals (seconds) from sorted spike times.
pub fn inter_spike_intervals(spike_times: &[f64]) -> Vec<f64> {
    if spike_times.len() < 2 {
        return Vec::new();
    }
    let mut sorted = spike_times.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    sorted.windows(2).map(|w| w[1] - w[0]).collect()
}

/// Histogram of inter-spike intervals.
pub fn isi_histogram(spike_times: &[f64], bin_width: f64) -> IsiHistogram {
    assert!(bin_width > 0.0, "bin_width must be positive");

    let isis = inter_spike_intervals(spike_times);
    if isis.is_empty() {
        return IsiHistogram {
            bin_edges: vec![0.0],
            counts: vec![],
        };
    }

    let max_isi = isis.iter().copied().fold(0.0f64, f64::max);
    let n_bins = (max_isi / bin_width).ceil() as usize + 1;
    let mut counts = vec![0u64; n_bins.max(1)];
    let bin_edges: Vec<f64> = (0..n_bins)
        .map(|i| i as f64 * bin_width)
        .collect();

    for isi in isis {
        if isi < 0.0 {
            continue;
        }
        let bin = (isi / bin_width) as usize;
        if bin < counts.len() {
            counts[bin] += 1;
        }
    }

    IsiHistogram { bin_edges, counts }
}
