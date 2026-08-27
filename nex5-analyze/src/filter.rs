/// Inclusive time window in seconds (same units as nex5 timestamps).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeRange {
    pub start: f64,
    pub end: f64,
}

impl TimeRange {
    pub fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }

    pub fn contains(&self, t: f64) -> bool {
        t >= self.start && t <= self.end
    }
}

/// Keep timestamps inside `range` (inclusive).
pub fn filter_timestamps(timestamps: &[f64], range: TimeRange) -> Vec<f64> {
    timestamps
        .iter()
        .copied()
        .filter(|t| range.contains(*t))
        .collect()
}

/// Filter a neuron's spike train by time range.
pub fn filter_file_neuron(
    data: &nex5file::FileData,
    name: &str,
    range: TimeRange,
) -> nex5file::Result<Vec<f64>> {
    let nr = data.neuron(name)?;
    Ok(filter_timestamps(&nr.timestamps.as_f64_vec(), range))
}

/// Filter an event variable's timestamps by time range.
pub fn filter_file_events(
    data: &nex5file::FileData,
    name: &str,
    range: TimeRange,
) -> nex5file::Result<Vec<f64>> {
    let ev = data.event(name)?;
    Ok(filter_timestamps(&ev.timestamps.as_f64_vec(), range))
}
