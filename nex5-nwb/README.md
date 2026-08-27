# nex5-nwb

Convert between [`nex5file`](../) in-memory [`FileData`](https://docs.rs/nex5file/latest/nex5file/struct.FileData.html) and [NWB 2.x](https://www.nwb.org/) HDF5 files via [`consus-nwb`](https://crates.io/crates/consus-nwb).

## Mapping

| NeuroExplorer | NWB 2.x |
|---------------|---------|
| Neuron spike trains | `Units` table (`spike_times` + `spike_times_index`) |
| Events | `acquisition/{name}` `TimeSeries` (timestamps only) |
| Continuous (A/D) | `acquisition/{name}` `TimeSeries` (`starting_time` + `rate`) |

Intervals, markers, waveforms, and population vectors are preserved only on the nex5 side today.

## Example

```rust
use nex5file::FileData;
use nex5_nwb::{read_nwb_file, write_nwb_file, NwbReadOptions, NwbWriteOptions};

let data = read_nwb_file("session.nwb", &NwbReadOptions::default())?;
write_nwb_file(&data, "copy.nwb", &NwbWriteOptions::default())?;
```
