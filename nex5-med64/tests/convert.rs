use med64::AnalysisOptions;
use nex5_med64::{modat_to_file_data, Med64ConvertOptions};
use std::path::PathBuf;

fn sample_modat() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../med64/data/sample data1.modat");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

#[test]
fn med64_modat_converts_to_nex5_neurons() {
    let Some(path) = sample_modat() else {
        eprintln!("skipping med64 test: sample data1.modat not found");
        return;
    };
    let opts = Med64ConvertOptions {
        max_blocks: Some(1),
        analysis: AnalysisOptions {
            max_blocks: Some(1),
            ..Default::default()
        },
        ..Med64ConvertOptions::default()
    };
    let data = modat_to_file_data(&path, &opts).expect("convert");
    assert!(!data.neuron_names().is_empty());
}
