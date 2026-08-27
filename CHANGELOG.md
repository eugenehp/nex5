# Changelog

All notable changes to this project are documented in this file.

## [1.2.0] - 2026-08-27

### Added

- **`Timestamps`** storage type with optional **`compact_timestamps`** read option (`f32` in-memory)
- **`parallel_decode`** — rayon parallel variable payload reads from disk (requires `parallel` feature)
- **`FileData::diff`** — compare sessions by variable name/count
- **Async write** — `write_nex5_file_async`, `write_with_options_async`
- **`nex5-cli`** — `info`, `export-spikes`, `to-nwb`, `from-nwb`, `psth` commands
- **`nex5-med64`** — MED64 `.modat` → nex5 converter
- **NWB** — neuron name preservation, interval round-trip via tagged TimeSeries pairs
- **`nex5-analyze`** — raster, cross-correlation, smoothed firing rate, `analyze_file` helper
- **Expanded `nex5file-py`** — neuron/continuous accessors and `add_neuron`
- **NWB markers & waveforms** — round-trip via tagged TimeSeries + session JSON payload
- **Zero-copy mmap views** — `MmapTimestampsView`, `MmapWaveformSamplesView`, `OpenNexFile::mmap_variable_view`
- **`nex5-sort`** — Kilosort-style CPU pipeline + Phy/Kilosort `.npy` import/export
- **`nex5-cli`** — `sort`, `import-phy` subcommands

### Improved (1.2)

- **Sorting** — abs-peak detection, refractory period, template refinement, multichannel CAR fix, i64 spike_times import, noise-cluster skip
- **Mmap views** — `wave_f32`, `sample_f32`, indexed timestamp access, i16 scaling via `ad_to_mv`
- **NWB waveforms** — sample arrays stored with real `sampling_rate` metadata

### Changed

- Spike/event/marker/waveform timestamps use [`Timestamps`](src/variables/timestamps.rs) (API: `.as_f64_vec()`, `PartialEq` with `Vec<f64>`)
- Criterion bench clippy fix
- **`nex5-med64`** depends on [`med64`](https://crates.io/crates/med64) `0.0.2` from crates.io (no longer a local path)
- Workspace crates use versioned path deps (`nex5file = { version = "1.2.0", path = ".." }`) so they can be published to crates.io
- All workspace crates aligned at **1.2.0**
- **`nex5`** umbrella crate — prelude + feature flags for `analyze` / `nwb` / `sort` / `med64` / `full`

### Added (workspace)

- **`nex5-nwb`** and **`nex5-analyze`** sibling crates
- **`nex5-nwb`** — read/write NWB 2.x via `consus-nwb`; maps neurons → `Units`, events/continuous → `TimeSeries`
- **`nex5-analyze`** — PSTH, time-range filtering, ISI histograms, spike-time sort / unit rate ranking

## [1.1.0] - 2026-08-27

### Added

- **Streaming reads** — `TimestampStream`, `OpenNexFile::stream_timestamps`, `load_timestamps_range`, `load_timestamps_first`
- **Chunked timestamp I/O** — `for_each_timestamp_chunk`, `read_timestamps_range` in format layer
- **`FileDataBuilder`** — fluent construction API
- **`FileData::merge`**, **`subset`**, **`rename_variable`**
- **Export** — `export_spikes`, `SpikeExportFormat`, `export_spikes_to_file`
- **Typed metadata** — `FileMetadata`, `VariableMetadata`, `FileData::file_metadata()`
- **`fixture_paths`** module + generated `tests/fixtures/*.nex5`
- **Criterion benchmarks** — `benches/roundtrip.rs`
- **Optional features**: `async` (tokio read), `mmap` (`OpenNexFile::open_mmap`, `MmapReader`), `parallel` (reserved), `full`
- **`nex5file-py`** — optional PyO3 bindings crate
- **`no_std`** (from 1.0.x unreleased) — `read_from_slice` / `write_to_vec`

### Changed

- Version **1.1.0**; MSRV remains 1.74
- `unsafe_code` allowed only when `mmap` feature is enabled

## [1.0.0] - 2026-08-27

### Added

- `OpenNexFile` — cached file handle for efficient lazy loading via `load_variables()`
- `Reader::read_from_reader()` — read from any `Read + Seek` source
- `Reader::open_file()` / `open_headers_only()`
- `WriteOptions` — buffer size, metadata embedding, writer identity in JSON metadata
- `ReadOptions::sequential_io` — read payloads in on-disk order
- `FileData` name index — O(1) variable lookup
- `IndexMut` and typed mut accessors (`event_mut`, `neuron_mut`, etc.)
- Interval validation (`InvalidInterval` error)
- Write guard: errors if saving with unloaded variable payloads
- Examples: `read_headers`, `convert_to_nex`, `extract_spikes`
- Chunked bulk I/O for reads and writes (lower peak memory, fewer syscalls)

### Changed

- **Breaking:** `WaveformVariable` stores samples in a flat `Vec<f32>` (use `wave(n)` or `waveform_values_nested()`)
- Selective variable reads retain **all** variable headers (only payloads are filtered)
- `read_nex_file_variables` accepts `&[&str]` / `AsRef<str>` instead of `&[String]`
- `Writer` uses `WriteOptions` and embeds crate version in metadata
- Bumped MSRV remains 1.74

### Fixed

- `.nex` variable header padding on read (52-byte field)
- Timestamp scaling precision (divide vs multiply-by-inverse)

## [0.2.0] - 2026-08-27

Initial Rust port with immutable writer, read validation, lazy loading, and comprehensive tests.

## [0.1.0]

Early prototype.
