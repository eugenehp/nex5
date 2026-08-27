# nex5

Unified façade for the NeuroExplorer `.nex` / `.nex5` toolkit.

```toml
[dependencies]
nex5 = { version = "1.2", features = ["full"] }
```

```rust
use nex5::prelude::*;

fn main() -> nex5file::Result<()> {
    let data = read_nex5_file("recording.nex5")?;
    println!("{} neurons", data.neuron_names().len());
    Ok(())
}
```

## Features

| Feature | Enables | Default |
|---------|---------|---------|
| `std` | `nex5file/std` | yes |
| `async` | async nex5file I/O | no |
| `mmap` | memory-mapped reads | no |
| `parallel` | parallel decode | no |
| `file-full` | `nex5file` `full` | no |
| `analyze` | [`nex5-analyze`](https://crates.io/crates/nex5-analyze) | no |
| `nwb` | [`nex5-nwb`](https://crates.io/crates/nex5-nwb) | no |
| `sort` | [`nex5-sort`](https://crates.io/crates/nex5-sort) | no |
| `med64` | [`nex5-med64`](https://crates.io/crates/nex5-med64) | no |
| `full` | all library features + `file-full` | no |

Modules when features are on: `nex5::analyze`, `nex5::nwb`, `nex5::sort`, `nex5::med64`, plus always-on `nex5::nex5file` via `use nex5file` re-export.

CLI (separate crate): `cargo install nex5-cli`.
