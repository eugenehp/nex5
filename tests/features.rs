//! Integration tests for new 1.1 features.

use nex5file::{
    export_spikes, fixture_paths, FileData, FileDataBuilder, NexFormat, OpenNexFile,
    Reader, SpikeExportOptions, TimestampRange, Writer,
};

fn write_minimal_fixtures() {
    let dir = fixture_paths::fixtures_dir();
    let _ = std::fs::create_dir_all(&dir);
    let data = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .comment("minimal fixture")
        .event("stim", vec![0.001, 0.002, 0.003])
        .unwrap()
        .neuron("unit_a", vec![0.010, 0.020], 1, 1, 0.0, 0.0)
        .unwrap()
        .build()
        .unwrap();
    Writer::new()
        .write_nex5_file(&data, fixture_paths::minimal_nex5())
        .unwrap();
    Writer::new()
        .write_nex5_file(&data, fixture_paths::events_neurons_nex5())
        .unwrap();
}

#[test]
fn builder_and_subset_merge_rename() {
    let mut a = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .event("a", vec![1.0])
        .unwrap()
        .build()
        .unwrap();
    let b = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .event("b", vec![2.0])
        .unwrap()
        .build()
        .unwrap();
    a.merge(&b).unwrap();
    assert_eq!(a.event_names().len(), 2);
    a.rename_variable("a", "alpha").unwrap();
    let sub = a.subset(&["alpha"]).unwrap();
    assert_eq!(sub.event_names(), vec!["alpha"]);
}

#[test]
fn stream_timestamps_without_full_load() {
    write_minimal_fixtures();
    let path = fixture_paths::minimal_nex5();
    let mut open = OpenNexFile::open_headers_only(&path).unwrap();
    let mut collected = Vec::new();
    open.stream_timestamps("stim", |chunk| {
        collected.extend_from_slice(chunk);
        Ok(())
    })
    .unwrap();
    assert_eq!(collected, vec![0.001, 0.002, 0.003]);
}

#[test]
fn load_timestamps_range_partial() {
    write_minimal_fixtures();
    let path = fixture_paths::minimal_nex5();
    let mut open = OpenNexFile::open_headers_only(&path).unwrap();
    let ts = open
        .load_timestamps_range("stim", TimestampRange::from_start_count(1, 2))
        .unwrap();
    assert_eq!(ts, vec![0.002, 0.003]);
}

#[test]
fn export_spikes_csv() {
    let data = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .event("spikes", vec![0.1, 0.2])
        .unwrap()
        .build()
        .unwrap();
    let mut buf = Vec::new();
    export_spikes(&data, "spikes", &mut buf, &SpikeExportOptions::new()).unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(text.contains("0.1"));
    assert!(text.contains("0.2"));
}

#[test]
fn metadata_typed_accessors() {
    let mut data = FileData::new(100_000.0, "").unwrap();
    data.metadata = serde_json::json!({
        "file": { "writerSoftware": { "name": "nex5file", "version": "1.2.0" } },
        "variables": [{ "name": "unit_a", "unitNumber": 3 }]
    });
    let meta = data.file_metadata();
    assert_eq!(meta.writer_name.as_deref(), Some("nex5file"));
    assert_eq!(
        data.variable_metadata("unit_a").unwrap().unit_number,
        Some(3)
    );
}

#[test]
fn fixture_roundtrip_on_disk() {
    write_minimal_fixtures();
    let back = Reader::new()
        .read_nex5_file(fixture_paths::minimal_nex5())
        .unwrap();
    assert_eq!(back.event("stim").unwrap().timestamps.len(), 3);
}

#[test]
fn read_from_slice_matches_file() {
    write_minimal_fixtures();
    let bytes = std::fs::read(fixture_paths::minimal_nex5()).unwrap();
    let from_slice = Reader::new()
        .read_from_slice(&bytes, NexFormat::Nex5)
        .unwrap();
    let from_file = Reader::new()
        .read_nex5_file(fixture_paths::minimal_nex5())
        .unwrap();
    assert_eq!(
        from_slice.event("stim").unwrap().timestamps,
        from_file.event("stim").unwrap().timestamps
    );
}

#[cfg(feature = "mmap")]
#[test]
fn mmap_timestamps_view_zero_copy() {
    write_minimal_fixtures();
    let open = OpenNexFile::open_mmap(fixture_paths::minimal_nex5()).unwrap();
    let view = open.mmap_timestamps_view("stim").unwrap();
    let ts: Vec<f64> = view.iter_seconds().collect();
    assert_eq!(ts, vec![0.001, 0.002, 0.003]);
}

#[cfg(feature = "mmap")]
#[test]
fn mmap_waveform_view_decodes_samples() {
    let mut data = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .build()
        .unwrap();
    data.add_wave_var_with_floats(
        "wf",
        20_000.0,
        vec![0.01],
        vec![vec![1.0, 2.0, 3.0]],
    )
    .unwrap();
    let path = fixture_paths::fixtures_dir().join("mmap_wave.nex5");
    Writer::new().write_nex5_file(&data, &path).unwrap();
    let open = OpenNexFile::open_mmap(&path).unwrap();
    match open.mmap_variable_view("wf").unwrap() {
        nex5file::MmapVariableView::Waveform { samples, .. } => {
            assert_eq!(samples.num_waves(), 1);
            assert_eq!(samples.wave_f32(0).unwrap(), vec![1.0, 2.0, 3.0]);
        }
        other => panic!("expected waveform view, got {other:?}"),
    }
}

#[cfg(feature = "mmap")]
#[test]
fn mmap_open_reads_events() {
    write_minimal_fixtures();
    let open = OpenNexFile::open_mmap(fixture_paths::minimal_nex5()).unwrap();
    assert!(open.data().event_names().contains(&"stim".to_string()));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_read_matches_sync() {
    write_minimal_fixtures();
    let path = fixture_paths::minimal_nex5();
    let async_data = nex5file::read_nex5_file_async(&path).await.unwrap();
    let sync_data = Reader::new().read_nex5_file(&path).unwrap();
    assert_eq!(
        async_data.event("stim").unwrap().timestamps,
        sync_data.event("stim").unwrap().timestamps
    );
}
