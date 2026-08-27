//! Convenient re-exports for everyday nex5 usage.
//!
//! ```
//! use nex5::prelude::*;
//! ```

pub use nex5file::{
    ContinuousVariable, EventVariable, FileData, IntervalVariable, MarkerFieldValue, MarkerVariable,
    NeuronVariable, NexError, NexFormat, NexFileVarType, PopulationVector, ReadOptions, Reader,
    Result, Timestamps, Variable, WaveformVariable, WriteOptions, Writer, NEX5_MAGIC, NEX_MAGIC,
};

#[cfg(feature = "std")]
pub use nex5file::{
    export_spikes, export_spikes_to_file, read_nex5_file, read_nex_file, write_nex5_file,
    write_nex_file, FileDataBuilder, FileMetadata, OpenNexFile, SpikeExportFormat,
    SpikeExportOptions, VariableMetadata,
};

#[cfg(feature = "mmap")]
pub use nex5file::{
    MmapContinuousSamplesView, MmapTimestampsView, MmapVariableView, MmapWaveformSamplesView,
};

#[cfg(feature = "async")]
pub use nex5file::{
    read_nex5_file_async, read_nex_file_async, write_nex5_file_async, write_nex_file_async,
};

#[cfg(feature = "analyze")]
pub use nex5_analyze::{
    analyze_file, filter_file_events, filter_file_neuron, filter_timestamps,
    inter_spike_intervals, isi_histogram, peri_event_raster, psth, smooth_firing_rate,
    sort_spike_times, spike_cross_correlation, units_by_mean_rate, CrossCorrelationResult,
    FileAnalysisOptions, FileAnalysisResult, IsiHistogram, PsthOptions, PsthResult, RasterResult,
    TimeRange, TrialRaster,
};

#[cfg(feature = "nwb")]
pub use nex5_nwb::{
    read_nwb_bytes, read_nwb_file, write_nwb_bytes, write_nwb_file, NwbError, NwbReadOptions,
    NwbWriteOptions,
};

#[cfg(feature = "sort")]
pub use nex5_sort::{
    phy_to_file_data, write_phy_folder, KilosortPipeline, KilosortPipelineOptions, PhyImportOptions,
    SortError, SortResult,
};

#[cfg(feature = "med64")]
pub use nex5_med64::{modat_to_file_data, modat_to_nex5, Med64ConvertOptions};
