use nex5::prelude::*;

#[test]
fn core_prelude_builds_file_data() {
    let data = FileData::new(40_000.0, "umbrella").unwrap();
    assert!(data.variables.is_empty());
    assert_eq!(nex5::VERSION, "1.2.0");
}

#[cfg(feature = "analyze")]
#[test]
fn analyze_feature_exports_psth() {
    let spikes = vec![0.01, 0.02];
    let events = vec![0.0];
    let out = psth(&spikes, &events, &PsthOptions::default());
    assert!(!out.counts.is_empty());
}
