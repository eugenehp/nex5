//! Spike-train analysis on top of [`nex5file`].
//!
//! - **PSTH** — peri-event spike histograms
//! - **Filtering** — time-range selection on timestamps and file variables
//! - **ISI** — inter-spike interval histograms
//! - **Sort** — order spike times; rank units by firing rate
//!
//! For NWB import/export use the sibling [`nex5-nwb`] crate.
//!
//! [`nex5-nwb`]: ../nex5_nwb/index.html

mod filter;
mod file;
mod isi;
mod psth;
mod raster;
mod correlation;
mod smooth;
mod sort;

pub use filter::{filter_file_events, filter_file_neuron, filter_timestamps, TimeRange};
pub use file::{analyze_file, FileAnalysisOptions, FileAnalysisResult};
pub use isi::{inter_spike_intervals, isi_histogram, IsiHistogram};
pub use psth::{psth, PsthOptions, PsthResult};
pub use raster::{peri_event_raster, RasterResult, TrialRaster};
pub use correlation::{spike_cross_correlation, CrossCorrelationResult};
pub use smooth::smooth_firing_rate;
pub use sort::{sort_spike_times, units_by_mean_rate};

pub type Result<T> = nex5file::Result<T>;
