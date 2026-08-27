use nex5file::FileData;
use nex5_nwb::{read_nwb_bytes, write_nwb_bytes, NwbReadOptions, NwbWriteOptions};

#[test]
fn nwb_roundtrip_neurons_events_continuous() {
    let mut original = FileData::new(100_000.0, "roundtrip test").unwrap();
    original
        .add_neuron("nr_a", vec![0.010, 0.020, 0.030], 1, 1, 10.0, 20.0)
        .unwrap();
    original
        .add_neuron("nr_b", vec![0.015, 0.025], 2, 2, 30.0, 40.0)
        .unwrap();
    original
        .add_event("stim", vec![0.100, 0.200, 0.300])
        .unwrap();
    original
        .add_interval_as_pairs_start_end("trial", &[(0.0, 0.5), (1.0, 1.5)])
        .unwrap();
    original
        .add_cont_var_with_floats_single_fragment(
            "lfp",
            1_000.0,
            0.0,
            vec![0.0, 0.1, -0.1, 0.2],
        )
        .unwrap();

    let nwb_bytes = write_nwb_bytes(&original, &NwbWriteOptions::default()).unwrap();
    let restored = read_nwb_bytes(&nwb_bytes, &NwbReadOptions::default()).unwrap();

    assert_eq!(restored.neuron_names(), ["nr_a", "nr_b"]);
    assert_eq!(restored.event_names(), ["stim"]);
    assert_eq!(restored.continuous_names(), ["lfp"]);

    let lfp = restored.continuous("lfp").unwrap();
    assert_eq!(lfp.continuous_values, vec![0.0, 0.1, -0.1, 0.2]);
    assert!((lfp.sampling_rate() - 1_000.0).abs() < f64::EPSILON);

    assert_eq!(restored.interval_names(), ["trial"]);
    let trial = restored.interval("trial").unwrap();
    assert_eq!(trial.interval_starts, vec![0.0, 1.0]);
    assert_eq!(trial.interval_ends, vec![0.5, 1.5]);

    let stim = restored.event("stim").unwrap();
    assert_eq!(stim.timestamps, vec![0.100, 0.200, 0.300]);

    let total_spikes: usize = restored
        .neuron_names()
        .iter()
        .map(|name| restored.neuron(name).unwrap().timestamps.len())
        .sum();
    assert_eq!(total_spikes, 5);
}

#[test]
fn nwb_roundtrip_markers_and_waveforms() {
    use nex5file::MarkerFieldValue;

    let mut original = FileData::new(40_000.0, "marker/wave test").unwrap();
    original
        .add_marker(
            "trials",
            vec![0.1, 0.2],
            vec!["code".into(), "note".into()],
            vec![
                vec![
                    MarkerFieldValue::Number(1),
                    MarkerFieldValue::Number(2),
                ],
                vec![
                    MarkerFieldValue::String("a".into()),
                    MarkerFieldValue::String("b".into()),
                ],
            ],
        )
        .unwrap();
    original
        .add_wave_var_with_floats(
            "spk_wf",
            20_000.0,
            vec![0.05, 0.15],
            vec![
                vec![1.0, 0.5, 0.0],
                vec![-1.0, -0.5, 0.0],
            ],
        )
        .unwrap();

    let nwb_bytes = write_nwb_bytes(&original, &NwbWriteOptions::default()).unwrap();
    let restored = read_nwb_bytes(&nwb_bytes, &NwbReadOptions::default()).unwrap();

    assert_eq!(restored.marker_names(), ["trials"]);
    let m = restored.marker("trials").unwrap();
    assert_eq!(m.timestamps.as_f64_vec(), vec![0.1, 0.2]);
    assert_eq!(m.marker_field_names, vec!["code", "note"]);
    assert_eq!(m.marker_fields[0][0], MarkerFieldValue::Number(1));
    assert_eq!(
        m.marker_fields[1][1],
        MarkerFieldValue::String("b".into())
    );

    assert_eq!(restored.wave_names(), ["spk_wf"]);
    let wf = restored.waveform("spk_wf").unwrap();
    assert_eq!(wf.timestamps.as_f64_vec(), vec![0.05, 0.15]);
    assert!((wf.sampling_rate() - 20_000.0).abs() < f64::EPSILON);
    assert_eq!(wf.waveform_values.len(), 6);
}

#[test]
fn nwb_write_read_empty_session() {
    let data = FileData::new(40_000.0, "empty").unwrap();
    let bytes = write_nwb_bytes(&data, &NwbWriteOptions::default()).unwrap();
    let restored = read_nwb_bytes(&bytes, &NwbReadOptions::default()).unwrap();
    assert!(restored.variables.is_empty());
}
