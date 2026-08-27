//! Read, write, and edit data stored in NeuroExplorer [`.nex`](https://www.neuroexplorer.com) and `.nex5` files.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "mmap"), forbid(unsafe_code))]

extern crate alloc;

mod compat;
mod error;
mod file_data;
mod format;
mod io_ext;
mod read_options;
mod reader;
mod stream;
mod validation;
mod variables;
mod write_options;
mod write_plan;
mod writer;

#[cfg(feature = "std")]
mod builder;
#[cfg(feature = "std")]
mod diff;
#[cfg(feature = "std")]
mod export;
#[cfg(feature = "std")]
pub mod fixture_paths;
#[cfg(feature = "std")]
mod metadata;
#[cfg(feature = "std")]
mod open_file;

#[cfg(feature = "mmap")]
mod mmap;
#[cfg(feature = "mmap")]
mod mmap_view;

#[cfg(feature = "async")]
mod async_io;

pub use error::{NexError, Result};
pub use file_data::FileData;
pub use read_options::ReadOptions;
pub use reader::{NexFormat, Reader};
pub use stream::{TimestampRange, TimestampStream, validate_timestamp_range};
pub use variables::{
    ContinuousVariable, EventVariable, IntervalVariable, MarkerFieldValue, MarkerVariable,
    NeuronVariable, NexFileVarType, PopulationVector, Timestamps, Variable, WaveformVariable,
};
pub use write_options::WriteOptions;
pub use write_plan::{
    ensure_all_payloads_loaded, prepare_nex5_write_plan, prepare_nex_write_plan, FileWritePlan,
    VariableWritePlan,
};
pub use writer::Writer;

#[cfg(feature = "std")]
pub use diff::{FileDataDiff, VariableDiff};
#[cfg(feature = "std")]
pub use builder::FileDataBuilder;
#[cfg(feature = "std")]
pub use export::{export_spikes, export_spikes_to_file, spike_export_names, SpikeExportFormat, SpikeExportOptions};
#[cfg(feature = "std")]
pub use metadata::{FileMetadata, VariableMetadata};
#[cfg(feature = "std")]
pub use open_file::OpenNexFile;

#[cfg(feature = "mmap")]
pub use mmap::MmapReader;
#[cfg(feature = "mmap")]
pub use mmap_view::{
    MmapContinuousSampleIter, MmapContinuousSamplesView, MmapTimestampIter, MmapTimestampsView,
    MmapVariableView, MmapWaveIter, MmapWaveformSamplesView,
};

#[cfg(feature = "async")]
pub use async_io::{
    read_nex5_file_async, read_nex_file_async, read_with_options_async, write_nex5_file_async,
    write_nex_file_async, write_with_options_async,
};

#[cfg(feature = "std")]
pub use reader::{read_nex5_file, read_nex_file};

#[cfg(feature = "std")]
pub use writer::{write_nex5_file, write_nex5_file_with_options, write_nex_file};

pub use format::{NEX5_MAGIC, NEX_MAGIC};
