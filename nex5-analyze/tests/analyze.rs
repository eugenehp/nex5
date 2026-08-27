use nex5_analyze::{
    filter_timestamps, inter_spike_intervals, isi_histogram, psth, sort_spike_times,
    units_by_mean_rate, PsthOptions, TimeRange,
};
use nex5file::FileData;

#[test]
fn filter_and_sort_spikes() {
    let ts = vec![0.01, 0.05, 0.10, 0.20];
    let filtered = filter_timestamps(&ts, TimeRange::new(0.04, 0.15));
    assert_eq!(filtered, vec![0.05, 0.10]);
    assert_eq!(sort_spike_times(&[0.2, 0.1, 0.3]), vec![0.1, 0.2, 0.3]);
}

#[test]
fn isi_histogram_counts_intervals() {
    let spikes = vec![0.0, 0.01, 0.03, 0.06];
    let isis = inter_spike_intervals(&spikes);
    assert_eq!(isis.len(), 3);
    assert!((isis[0] - 0.01).abs() < 1e-9);
    assert!((isis[1] - 0.02).abs() < 1e-9);
    assert!((isis[2] - 0.03).abs() < 1e-9);

    let hist = isi_histogram(&spikes, 0.01);
    assert_eq!(hist.counts.len(), hist.bin_edges.len());
    assert!(hist.counts.iter().sum::<u64>() >= 3);
}

#[test]
fn psth_aligns_spikes_to_events() {
    let spikes = vec![0.10, 0.15, 0.25, 0.35];
    let events = vec![0.10, 0.30];
    let opts = PsthOptions {
        window_start: 0.0,
        window_end: 0.10,
        bin_width: 0.05,
    };
    let result = psth(&spikes, &events, &opts);
    assert_eq!(result.n_events, 2);
    assert_eq!(result.counts.len(), 2);
    assert!(result.counts.iter().sum::<u64>() >= 2);
}

#[test]
fn units_by_mean_rate_orders_neurons() {
    let mut data = FileData::new(100_000.0, "").unwrap();
    data.add_neuron("slow", vec![0.1, 0.2], 0, 0, 0.0, 0.0)
        .unwrap();
    data.add_neuron("fast", vec![0.1, 0.2, 0.3, 0.4], 0, 1, 0.0, 0.0)
        .unwrap();
    data.end_seconds = 1.0;

    let ranked = units_by_mean_rate(&data);
    assert_eq!(ranked[0].0, "fast");
    assert!(ranked[0].1 > ranked[1].1);
}

#[test]
fn filter_file_neuron_integration() {
    let mut data = FileData::new(100_000.0, "").unwrap();
    data.add_neuron("u1", vec![0.01, 0.05, 0.09], 0, 0, 0.0, 0.0)
        .unwrap();
    let filtered = nex5_analyze::filter_file_neuron(&data, "u1", TimeRange::new(0.04, 0.08)).unwrap();
    assert_eq!(filtered, vec![0.05]);
}

#[test]
fn raster_and_correlation_helpers() {
    use nex5_analyze::{
        peri_event_raster, smooth_firing_rate, spike_cross_correlation, PsthOptions,
    };
    let spikes = vec![0.10, 0.15, 0.25];
    let events = vec![0.10, 0.30];
    let raster = peri_event_raster(&spikes, &events, &PsthOptions::default());
    assert_eq!(raster.trials.len(), 2);
    let xcorr = spike_cross_correlation(&spikes, &spikes, 0.1, 0.01);
    assert!(!xcorr.counts.is_empty());
    let smoothed = smooth_firing_rate(&[1, 2, 1], 0.01, 1.0);
    assert_eq!(smoothed.len(), 3);
}
