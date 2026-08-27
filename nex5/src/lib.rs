//! Unified façade for the nex5 NeuroExplorer toolkit.
//!
//! Enable optional crates with Cargo features:
//!
//! | Feature | Crate | Purpose |
//! |---------|-------|---------|
//! | *(always)* | [`nex5file`] | Read/write `.nex` / `.nex5` |
//! | `analyze` | [`nex5-analyze`] | PSTH, ISI, raster, filtering |
//! | `nwb` | [`nex5-nwb`] | NWB 2.x import/export |
//! | `sort` | [`nex5-sort`] | Kilosort-style sorting + Phy I/O |
//! | `med64` | [`nex5-med64`] | MED64 `.modat` → nex5 |
//! | `full` | all of the above + `nex5file/full` | |
//!
//! ```toml
//! nex5 = { version = "1.2", features = ["analyze", "nwb"] }
//! # or
//! nex5 = { version = "1.2", features = ["full"] }
//! ```
//!
//! Prefer [`prelude`] for a single import of the common surface.
//!
//! The CLI is separate: `cargo install nex5-cli`.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod prelude;

/// Core `.nex` / `.nex5` I/O ([`nex5file`]).
pub use nex5file;

/// Spike-train analysis ([`nex5-analyze`]).
#[cfg(feature = "analyze")]
#[cfg_attr(docsrs, doc(cfg(feature = "analyze")))]
pub use nex5_analyze as analyze;

/// NWB 2.x conversion ([`nex5-nwb`]).
#[cfg(feature = "nwb")]
#[cfg_attr(docsrs, doc(cfg(feature = "nwb")))]
pub use nex5_nwb as nwb;

/// Spike sorting and Phy/Kilosort I/O ([`nex5-sort`]).
#[cfg(feature = "sort")]
#[cfg_attr(docsrs, doc(cfg(feature = "sort")))]
pub use nex5_sort as sort;

/// MED64 `.modat` conversion ([`nex5-med64`]).
#[cfg(feature = "med64")]
#[cfg_attr(docsrs, doc(cfg(feature = "med64")))]
pub use nex5_med64 as med64;

/// Crate version string (`1.2.1`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
