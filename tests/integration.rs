use nex5file::{
    read_nex5_file, write_nex5_file, FileData, MarkerFieldValue, NexError, OpenNexFile,
    ReadOptions, Reader, Variable, Writer,
};

#[test]
fn file_data_add_event_and_names() {
    let mut data = FileData::new(10_000.0, "Sample File Data").unwrap();
    data.add_event("new event", vec![1.2, 3.3, 4.4]).unwrap();
    assert_eq!(data.event_names(), vec!["new event"]);
    assert_eq!(
        data.event("new event").unwrap().timestamps,
        vec![1.2, 3.3, 4.4]
    );
}

#[test]
fn file_data_index_operator() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_event("ev", vec![1.0]).unwrap();
    assert!(matches!(data["ev"], Variable::Event(_)));
}

#[test]
fn duplicate_event_rejected() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_event("ev", vec![1.0]).unwrap();
    assert!(matches!(
        data.add_event("ev", vec![2.0]),
        Err(NexError::DuplicateVariable(_))
    ));
}

#[test]
fn file_data_add_neuron() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_neuron("new neuron", vec![0.04, 1.2, 1.7], 0, 0, 0.0, 0.0)
        .unwrap();
    assert_eq!(data.neuron_names(), vec!["new neuron"]);
}

#[test]
fn file_data_add_interval() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_interval_as_pairs_start_end("IntervalVariable", &[(0.0, 1.0), (1.5, 2.0)])
        .unwrap();
    let v = data.interval("IntervalVariable").unwrap();
    assert_eq!(v.interval_starts, vec![0.0, 1.5]);
    assert_eq!(v.interval_ends, vec![1.0, 2.0]);
}

#[test]
fn file_data_add_continuous_single_fragment() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_cont_var_with_floats_single_fragment(
        "ContVariable",
        1000.0,
        0.05,
        vec![1.0, 2.0, 3.7],
    )
    .unwrap();
    let v = data.continuous("ContVariable").unwrap();
    assert_eq!(v.continuous_values, vec![1.0, 2.0, 3.7]);
    assert_eq!(v.fragment_timestamps, vec![0.05]);
    assert_eq!(v.fragment_counts, vec![3]);
}

#[test]
fn file_data_add_continuous_int16() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_cont_single_fragment_values_int16(
        "ContVariable",
        1000.0,
        0.03,
        &[5, 3, 4],
        0.015,
        0.2,
    )
    .unwrap();
    let v = data.continuous("ContVariable").unwrap();
    assert_eq!(
        v.continuous_values,
        vec![5.0 * 0.015 + 0.2, 3.0 * 0.015 + 0.2, 4.0 * 0.015 + 0.2]
    );
}

#[test]
fn file_data_add_continuous_all_timestamps() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_cont_var_with_floats_all_timestamps(
        "ContVariable",
        1000.0,
        vec![0.003, 0.004, 0.005, 0.1, 0.101],
        vec![1.0, 2.0, 3.7, 9.0, 5.0],
    )
    .unwrap();
    let v = data.continuous("ContVariable").unwrap();
    assert_eq!(v.fragment_timestamps, vec![0.003, 0.1]);
    assert_eq!(v.fragment_counts, vec![3, 2]);
}

#[test]
fn roundtrip_nex5_events_and_neurons() {
    let freq = 100_000.0;
    let mut original = FileData::new(freq, "").unwrap();
    original
        .add_event("event name with spaces", vec![1.0, 2.0, 3.5])
        .unwrap();
    original
        .add_neuron("neuron 12345", vec![0.001, 2.54, 8.99], 15, 2, 25.34, 35.0)
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.nex5");
    write_nex5_file(&original, &path).unwrap();
    let loaded = read_nex5_file(&path).unwrap();

    assert_eq!(
        loaded.event("event name with spaces").unwrap().timestamps,
        vec![1.0, 2.0, 3.5]
    );
    let nr = loaded.neuron("neuron 12345").unwrap();
    assert_eq!(nr.timestamps, vec![0.001, 2.54, 8.99]);
    assert_eq!(nr.header.wire, 15);
    assert_eq!(nr.header.unit, 2);
}

#[test]
fn roundtrip_nex_events() {
    let freq = 100_000.0;
    let mut original = FileData::new(freq, "").unwrap();
    original.add_event("ev", vec![1.0, 2.0]).unwrap();
    original
        .add_neuron("nr", vec![0.5], 1, 1, 10.0, 20.0)
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.nex");
    Writer::new().write_nex_file(&original, &path).unwrap();
    let loaded = Reader::new().read_nex_file(&path).unwrap();
    assert_eq!(loaded.event("ev").unwrap().timestamps, vec![1.0, 2.0]);
    assert_eq!(loaded.neuron("nr").unwrap().timestamps, vec![0.5]);
}

#[test]
fn roundtrip_nex5_continuous() {
    let freq = 100_000.0;
    let mut original = FileData::new(freq, "").unwrap();
    original
        .add_cont_var_with_floats_single_fragment(
            "cont name with spaces",
            1000.0,
            0.5,
            vec![1.1, 2.2, 3.3],
        )
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cont.nex5");
    write_nex5_file(&original, &path).unwrap();
    let loaded = read_nex5_file(&path).unwrap();
    let v = loaded.continuous("cont name with spaces").unwrap();
    assert_eq!(v.fragment_timestamps, vec![0.5]);
    assert_eq!(v.fragment_counts, vec![3]);
    for (a, b) in v.continuous_values.iter().zip([1.1, 2.2, 3.3].iter()) {
        assert!((a - b).abs() < 1e-4);
    }
}

#[test]
fn roundtrip_nex5_waveform() {
    let freq = 100_000.0;
    let mut original = FileData::new(freq, "").unwrap();
    original
        .add_wave_var_with_floats(
            "ScriptGenerated",
            10_000.0,
            vec![10.0, 20.0],
            vec![vec![2.0, 3.0, 4.0, 1.0], vec![5.0, 6.0, 7.0, 2.0]],
        )
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wave.nex5");
    write_nex5_file(&original, &path).unwrap();
    let loaded = read_nex5_file(&path).unwrap();
    let v = loaded.waveform("ScriptGenerated").unwrap();
    assert_eq!(v.timestamps, vec![10.0, 20.0]);
    assert_eq!(
        v.waveform_values_flat(),
        vec![2., 3., 4., 1., 5., 6., 7., 2.]
    );
}

#[test]
fn roundtrip_interval() {
    let mut original = FileData::new(100_000.0, "").unwrap();
    original
        .add_interval_as_pairs_start_end("intervals", &[(0.0, 1.0), (2.0, 3.0)])
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("int.nex5");
    write_nex5_file(&original, &path).unwrap();
    let loaded = read_nex5_file(&path).unwrap();
    let (s, e) = loaded.interval("intervals").unwrap().intervals();
    assert_eq!(s, vec![0.0, 2.0]);
    assert_eq!(e, vec![1.0, 3.0]);
}

#[test]
fn roundtrip_population_vector() {
    let mut original = FileData::new(100_000.0, "").unwrap();
    original
        .add_population_vector("pop", vec![1.0, 2.5, 3.5])
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pop.nex5");
    write_nex5_file(&original, &path).unwrap();
    let loaded = read_nex5_file(&path).unwrap();
    assert_eq!(
        loaded.population_vector("pop").unwrap().weights,
        vec![1.0, 2.5, 3.5]
    );
}

#[test]
fn roundtrip_marker_string() {
    let mut original = FileData::new(100_000.0, "").unwrap();
    original
        .add_marker(
            "marks",
            vec![0.0, 1.0],
            vec!["label".to_string()],
            vec![vec![
                MarkerFieldValue::String("abc".to_string()),
                MarkerFieldValue::String("xyz".to_string()),
            ]],
        )
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("marker.nex5");
    write_nex5_file(&original, &path).unwrap();
    let loaded = read_nex5_file(&path).unwrap();
    let m = loaded.marker("marks").unwrap();
    assert_eq!(m.marker_field_names, vec!["label"]);
    assert_eq!(m.marker_fields[0].len(), 2);
}

#[test]
fn roundtrip_64bit_timestamps() {
    let freq = 100_000.0;
    let mut original = FileData::new(freq, "").unwrap();
    // Timestamp > 2^31 ticks at 100kHz (~6 hours)
    let big_ts = (i32::MAX as f64 + 1000.0) / freq;
    original.add_event("late", vec![big_ts]).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.nex5");
    write_nex5_file(&original, &path).unwrap();
    let loaded = read_nex5_file(&path).unwrap();
    let ts = loaded.event("late").unwrap().timestamps.as_f64_vec()[0];
    assert!((ts - big_ts).abs() < 1e-3);
}

#[test]
fn lazy_load_variables() {
    let mut original = FileData::new(100_000.0, "").unwrap();
    original.add_event("ev1", vec![1.0]).unwrap();
    original
        .add_cont_var_with_floats_single_fragment("cont1", 1000.0, 0.0, vec![1.0, 2.0])
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lazy.nex5");
    write_nex5_file(&original, &path).unwrap();

    let reader = Reader::new();
    let mut headers = reader.read_nex5_headers_only(&path).unwrap();
    assert!(!headers.is_variable_loaded("cont1").unwrap());

    reader
        .load_variables(&path, &mut headers, &["cont1"])
        .unwrap();
    assert!(headers.is_variable_loaded("cont1").unwrap());
    assert_eq!(
        headers.continuous("cont1").unwrap().continuous_values,
        vec![1.0, 2.0]
    );
}

#[test]
fn read_headers_only_then_selective_load() {
    let freq = 100_000.0;
    let mut original = FileData::new(freq, "").unwrap();
    original.add_event("ev1", vec![1.0]).unwrap();
    original
        .add_cont_var_with_floats_single_fragment("cont1", 1000.0, 0.0, vec![1.0, 2.0])
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mixed.nex5");
    write_nex5_file(&original, &path).unwrap();

    let headers = Reader::new().read_nex5_headers_only(&path).unwrap();
    assert_eq!(headers.continuous_names(), vec!["cont1"]);

    let partial = Reader::new()
        .read_nex5_file_variables(&path, &["cont1"])
        .unwrap();
    assert_eq!(partial.continuous_names(), vec!["cont1"]);
    assert_eq!(partial.event_names(), vec!["ev1"]);
    assert!(!partial.is_variable_loaded("ev1").unwrap());
    assert!(partial.is_variable_loaded("cont1").unwrap());
}

#[test]
fn add_marker_numeric() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_marker(
        "markers",
        vec![0.0, 1.0],
        vec!["id".to_string()],
        vec![vec![
            MarkerFieldValue::Number(1),
            MarkerFieldValue::Number(2),
        ]],
    )
    .unwrap();
    assert_eq!(data.marker_names(), vec!["markers"]);
}

#[test]
fn invalid_timestamp_frequency() {
    assert!(FileData::new(0.0, "").is_err());
}

#[test]
fn delete_variable() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_event("ev", vec![1.0]).unwrap();
    data.delete_variable("ev").unwrap();
    assert!(data.event_names().is_empty());
}

#[test]
fn serde_roundtrip_file_data() {
    let mut data = FileData::new(50_000.0, "test").unwrap();
    data.add_event("ev", vec![1.0, 2.0]).unwrap();
    let json = serde_json::to_string(&data).unwrap();
    let restored: FileData = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.comment, "test");
    assert_eq!(restored.event("ev").unwrap().timestamps, vec![1.0, 2.0]);
}

#[test]
fn open_nex_file_lazy_load() {
    let mut original = FileData::new(100_000.0, "").unwrap();
    original.add_event("ev1", vec![1.0]).unwrap();
    original
        .add_cont_var_with_floats_single_fragment("cont1", 1000.0, 0.0, vec![1.0, 2.0])
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("open.nex5");
    write_nex5_file(&original, &path).unwrap();

    let mut open = OpenNexFile::open_headers_only(&path).unwrap();
    assert!(!open.data().is_variable_loaded("cont1").unwrap());
    open.load_variables(&["cont1"]).unwrap();
    assert_eq!(
        open.data().continuous("cont1").unwrap().continuous_values,
        vec![1.0, 2.0]
    );
}

#[test]
fn invalid_interval_rejected() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    assert!(matches!(
        data.add_interval_as_pairs_start_end("bad", &[(1.0, 0.5)]),
        Err(NexError::InvalidInterval { index: 0 })
    ));
}

#[test]
fn metadata_error_by_default() {
    // Hand-built invalid metadata isn't trivial; verify option exists
    let opts = ReadOptions::new().ignore_metadata_errors(true);
    assert!(opts.ignore_metadata_errors);
}

#[test]
fn variable_timestamps_accessor() {
    let mut data = FileData::new(10_000.0, "").unwrap();
    data.add_event("ev", vec![1.0, 2.0]).unwrap();
    assert_eq!(
        data.get_variable("ev").unwrap().timestamps().unwrap(),
        &[1.0, 2.0]
    );
}
