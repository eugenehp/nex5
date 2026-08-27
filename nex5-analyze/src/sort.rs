use nex5file::{FileData, Variable};

/// Return a time-sorted copy of spike timestamps.
pub fn sort_spike_times(spike_times: &[f64]) -> Vec<f64> {
    let mut sorted = spike_times.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    sorted
}

/// List neuron variable names sorted by mean firing rate (descending).
///
/// Rate is estimated from spike count divided by the file's `[beg_seconds, end_seconds]` span.
pub fn units_by_mean_rate(data: &FileData) -> Vec<(String, f64)> {
    let duration = (data.end_seconds - data.beg_seconds).max(f64::EPSILON);
    let mut rates: Vec<(String, f64)> = data
        .variables
        .iter()
        .filter_map(|var| match var {
            Variable::Neuron(nr) => {
                let rate = nr.timestamps.len() as f64 / duration;
                Some((nr.header.name.clone(), rate))
            }
            _ => None,
        })
        .collect();
    rates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
    rates
}
