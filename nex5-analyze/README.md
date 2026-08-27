# nex5-analyze

Analysis utilities built on [`nex5file`](../): peri-stimulus histograms (PSTH), time-range filtering, inter-spike intervals (ISI), and basic spike-train sorting helpers.

Full spike sorting (e.g. Kilosort) is out of scope — use dedicated sorters and import results via `nex5file` or [`nex5-nwb`](../nex5-nwb).

## Example

```rust
use nex5_analyze::{psth, filter_timestamps, PsthOptions};
use nex5file::FileData;

let data = FileData::new(100_000.0, "")?;
let spikes = data.neuron("unit_a")?.timestamps.clone();
let events = data.event("stim")?.timestamps.clone();

let result = psth(&spikes, &events, &PsthOptions::default());
```
