//! Kilosort-style spike sorting and Phy/Kilosort folder I/O for [`nex5file`].
//!
//! This crate provides:
//! - **Phy/Kilosort import** — read `spike_times.npy` + `spike_clusters.npy` into [`nex5file::FileData`]
//! - **`KilosortPipeline`** — detect → extract templates → match → cluster (CPU, Kilosort-inspired)
//!
//! Full GPU Kilosort is not embedded; run Kilosort externally and import with [`phy::phy_to_file_data`].

mod error;
mod npy;
pub mod phy;
pub mod pipeline;

pub use error::{SortError, Result};
pub use phy::{phy_to_file_data, write_phy_folder, PhyImportOptions};
pub use pipeline::{KilosortPipeline, KilosortPipelineOptions, SortResult};
