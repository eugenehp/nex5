# nex5file

Rust library for reading, writing, and editing data stored in NeuroExplorer [`.nex`](https://www.neuroexplorer.com) and `.nex5` files.

This crate is a port of the Python [`nex5file`](https://github.com/NeuroExplorer/nex5file) package, with a matching feature set and API surface adapted to idiomatic Rust.

## Features

- Read and write `.nex` and `.nex5` files (format chosen by file extension)
- Variable types: neurons, events, intervals, markers, waveforms, continuous (A/D), population vectors
- Read full files, headers only, or selected variable payloads (all headers always retained)
- Lazy loading via [`OpenNexFile`](https://docs.rs/nex5file/latest/nex5file/struct.OpenNexFile.html) with a cached file handle
- Build and modify in-memory [`FileData`](https://docs.rs/nex5file/latest/nex5file/struct.FileData.html) before writing
- JSON metadata support in `.nex5` files
- Optional read validation (layout checks, payload size limits, sequential I/O)
- `serde` serialization of in-memory file data
- Chunked bulk I/O for large recordings
- **`no_std` feature** — read/write via in-memory buffers (`read_from_slice`, `write_to_vec`) without the standard library

## `no_std`

Disable default features for embedded or allocator-only environments:

```toml
nex5file = { version = "1", default-features = false, features = ["no_std"] }
```

```rust
use nex5file::{FileData, NexFormat, Reader, Writer};

let data = Reader::new().read_from_slice(&bytes, NexFormat::Nex5)?;
let out = Writer::new().write_to_vec(&data, NexFormat::Nex5)?;
```

See [`docs/format.md`](docs/format.md) for a binary format reference.

## Installation

```toml
[dependencies]
nex5file = "1.0"
```

Or from this repository:

```toml
nex5file = { path = "." }
```

## Quick start

```rust
use nex5file::{read_nex5_file, FileData, write_nex5_file};

fn main() -> nex5file::Result<()> {
    let data = read_nex5_file("recording.nex5")?;

    for name in data.continuous_names() {
        let cont = data.continuous(&name)?;
        println!(
            "{}: {} samples at {} Hz",
            name,
            cont.continuous_values.len(),
            cont.sampling_rate()
        );
    }

    let mut out = FileData::new(100_000.0, "my recording")?;
    out.add_event("stimuli", vec![0.001, 0.500, 1.000])?;
    out.add_neuron("unit_a", vec![0.012, 0.045, 0.102], 1, 1, 50.0, 50.0)?;

    write_nex5_file(&out, "output.nex5")?;
    Ok(())
}
```

## Lazy loading (large files)

```rust
use nex5file::OpenNexFile;

let mut open = OpenNexFile::open_headers_only("large.nex5")?;
println!("Channels: {:?}", open.data().continuous_names());

// Load only what you need — file handle stays open
open.load_variables(&["ad0", "ad1"])?;
let ad0 = open.data().continuous("ad0")?;
```

## Zero-copy mmap views (`mmap` feature)

When using `OpenNexFile::open_mmap`, decode timestamps and waveform bytes without allocating full vectors:

```rust
use nex5file::OpenNexFile;

let open = OpenNexFile::open_mmap("large.nex5")?;
let ts: Vec<f64> = open.mmap_timestamps_view("unit_a")?.iter_seconds().collect();
```

## Selective reads

Reading specific variables loads their payloads but **keeps every variable header**, so round-tripping a subset is safe as long as all payloads are loaded before write:

```rust
use nex5file::Reader;

let partial = Reader::new().read_nex5_file_variables("file.nex5", &["ad0"])?;
assert!(partial.continuous_names().contains(&"ad0".to_string()));
// Other variables appear in headers but are not loaded until requested
```

## Read / write options

```rust
use nex5file::{ReadOptions, WriteOptions, Reader, Writer};

let reader = Reader::with_options(
    ReadOptions::new()
        .sequential_io(true)
        .max_payload_bytes(512 * 1024 * 1024),
);

let writer = Writer::with_options(
    WriteOptions::new()
        .buffer_bytes(512 * 1024)
        .embed_metadata(true),
);
```

## Workspace crates

| Crate | Role |
|-------|------|
| **`nex5file`** (this crate) | Read/write `.nex` / `.nex5` |
| **[`nex5-nwb`](nex5-nwb/)** | Convert `FileData` ↔ NWB 2.x (Units, TimeSeries) |
| **[`nex5-analyze`](nex5-analyze/)** | PSTH, filtering, ISI, raster, cross-correlation |
| **[`nex5-sort`](nex5-sort/)** | Kilosort-style sorting + Phy/Kilosort `.npy` I/O |
| **[`nex5-cli`](nex5-cli/)** | `nex5` command: info, export, NWB convert, sort, PSTH |
| **[`nex5-med64`](nex5-med64/)** | MED64 `.modat` → `.nex5` converter |

NWB I/O uses [`consus-nwb`](https://crates.io/crates/consus-nwb) (pure Rust HDF5). Analysis stays separate so embedded/`no_std` builds are unaffected.

```toml
# Cargo.toml
nex5-nwb = { path = "nex5-nwb" }
nex5-analyze = { path = "nex5-analyze" }
```

```rust
use nex5_nwb::{read_nwb_file, write_nwb_file, NwbReadOptions, NwbWriteOptions};
use nex5_analyze::{psth, PsthOptions};

let data = read_nwb_file("session.nwb", &NwbReadOptions::default())?;
let psth = psth(
    &data.neuron("unit_0")?.timestamps,
    &data.event("stim")?.timestamps,
    &PsthOptions::default(),
);
write_nwb_file(&data, "copy.nwb", &NwbWriteOptions::default())?;
```

## Examples

```bash
cargo run --example read_headers -- recording.nex5
cargo run --example convert_to_nex -- recording.nex5 out.nex
cargo run --example extract_spikes -- recording.nex5 "unit_a" spikes.txt
```

## API mapping (Python → Rust)

| Python | Rust |
|--------|------|
| `Reader().ReadNex5File(path)` | `read_nex5_file(path)` |
| `Reader().ReadNexHeadersOnly(path)` | `Reader::new().read_nex_headers_only(path)` |
| `Reader().ReadNexFileVariables(path, names)` | `reader.read_nex5_file_variables(path, &names)` |
| `Writer().WriteNex5File(data, path)` | `write_nex5_file(&data, path)` |
| `data[name]` | `data.get_variable(name)?` or `data["name"]` |
| `data.AddEvent(...)` | `data.add_event(...)?` |
| `data.ContinuousNames()` | `data.continuous_names()` |

## MSRV

Rust **1.74** or later.

## License

MIT — same as the original [nex5file](https://github.com/NeuroExplorer/nex5file) project.
