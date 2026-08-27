use nex5file::FileData;

#[test]
fn compact_timestamps_use_f32_storage() {
    use nex5file::{fixture_paths, FileDataBuilder, NexFormat, ReadOptions, Reader, Writer};

    let dir = fixture_paths::fixtures_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = fixture_paths::minimal_nex5();
    let data = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .event("stim", vec![0.001, 0.002])
        .unwrap()
        .build()
        .unwrap();
    Writer::new().write_nex5_file(&data, &path).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    let opts = ReadOptions::new().compact_timestamps(true);
    let loaded = Reader::with_options(opts)
        .read_from_slice(&bytes, NexFormat::Nex5)
        .unwrap();
    assert!(loaded.event("stim").unwrap().timestamps.is_compact());
}

#[test]
fn file_data_diff_detects_subset() {
    let mut a = FileData::new(100_000.0, "").unwrap();
    a.add_event("ev", vec![1.0]).unwrap();
    let b = a.clone();
    let diff = a.diff(&b);
    assert!(diff.only_in_a.is_empty());
    assert!(diff.only_in_b.is_empty());
    assert!(diff.changed.is_empty());
}
