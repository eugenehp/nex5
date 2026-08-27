//! Run Kilosort-style sorting on synthetic traces and import Phy folders.

use nex5_sort::{phy_to_file_data, KilosortPipeline, KilosortPipelineOptions, PhyImportOptions};

#[test]
fn pipeline_and_phy_import() {
    let fs = 20_000.0;
    let n = (fs as usize) * 2;
    let mut trace = vec![0.0f32; n];
    for t in [0.2, 0.8] {
        let idx = (t * fs) as usize;
        for i in 0..25 {
            if idx + i < n {
                trace[idx + i] += 50.0 * (-((i as f32 - 12.0).powi(2)) / 18.0).exp();
            }
        }
    }
    let pipeline = KilosortPipeline::new(KilosortPipelineOptions {
        detect_threshold: 2.0,
        highpass_hz: 100.0,
        ..Default::default()
    });
    let sorted = pipeline.sort_traces(&[trace], fs).unwrap();
    assert!(!sorted.spike_times.is_empty());

    let dir = tempfile::tempdir().unwrap();
    let samples: Vec<f64> = sorted
        .spike_times
        .iter()
        .map(|t| t * fs)
        .collect();
    nex5_sort::phy::write_phy_folder(dir.path(), &samples, &sorted.spike_clusters).unwrap();
    let data = phy_to_file_data(
        dir.path(),
        &PhyImportOptions {
            sampling_rate: fs,
            timestamp_frequency_hz: fs,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!data.neuron_names().is_empty());
}
