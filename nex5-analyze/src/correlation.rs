/// Pairwise spike-time cross-correlation histogram (spike A relative to spike B).
#[derive(Debug, Clone, PartialEq)]
pub struct CrossCorrelationResult {
    pub bin_edges: Vec<f64>,
    pub counts: Vec<u64>,
}

/// Histogram of `(t_a - t_b)` for all spike pairs within `max_lag`.
pub fn spike_cross_correlation(
    spikes_a: &[f64],
    spikes_b: &[f64],
    max_lag: f64,
    bin_width: f64,
) -> CrossCorrelationResult {
    assert!(max_lag > 0.0 && bin_width > 0.0);

    let n_bins = ((2.0 * max_lag) / bin_width).ceil() as usize;
    let mut counts = vec![0u64; n_bins.max(1)];
    let bin_edges: Vec<f64> = (0..=n_bins)
        .map(|i| -max_lag + i as f64 * bin_width)
        .collect();

    if spikes_a.is_empty() || spikes_b.is_empty() {
        return CrossCorrelationResult { bin_edges, counts };
    }

    let mut a = spikes_a.to_vec();
    let mut b = spikes_b.to_vec();
    a.sort_by(|x, y| x.partial_cmp(y).unwrap_or(core::cmp::Ordering::Equal));
    b.sort_by(|x, y| x.partial_cmp(y).unwrap_or(core::cmp::Ordering::Equal));

    for &ta in &a {
        for &tb in &b {
            let lag = ta - tb;
            if lag.abs() > max_lag {
                continue;
            }
            let bin = ((lag + max_lag) / bin_width) as usize;
            if bin < counts.len() {
                counts[bin] += 1;
            }
        }
    }

    CrossCorrelationResult { bin_edges, counts }
}
