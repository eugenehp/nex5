//! Paths to bundled and sibling-repo test recordings.

use std::path::{Path, PathBuf};

/// Crate-local fixtures directory (`tests/fixtures/`).
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Sibling MED64 data directory (`../med64/data` from this repo).
pub fn med64_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../med64/data")
}

pub fn join_fixtures(relative: impl AsRef<Path>) -> PathBuf {
    fixtures_dir().join(relative)
}

pub fn minimal_nex5() -> PathBuf {
    join_fixtures("minimal.nex5")
}

pub fn events_neurons_nex5() -> PathBuf {
    join_fixtures("events_neurons.nex5")
}

pub fn med64_sample_data1() -> PathBuf {
    med64_data_dir().join("sample data1.modat")
}

/// True if path exists (for optional integration tests).
pub fn exists(path: &Path) -> bool {
    path.exists()
}
