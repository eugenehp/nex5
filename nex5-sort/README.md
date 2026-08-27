# nex5-sort

Kilosort-style spike sorting and Phy/Kilosort folder I/O for [`nex5file`](../).

- **`KilosortPipeline`** — band-pass → detect → PCA → cluster → template refine (CPU)
- **`phy_to_file_data`** — import `spike_times.npy` + `spike_clusters.npy` (supports `<f8>`, `<f4>`, `<i8>`, `<i4>`)
- **`SortResult::write_phy_folder`** — export back to Phy/Kilosort layout

Run Kilosort externally for GPU sorting, then import with `phy_to_file_data` or `nex5 import-phy`.

```rust
use nex5_sort::{KilosortPipeline, KilosortPipelineOptions, phy_to_file_data, PhyImportOptions};

let pipeline = KilosortPipeline::new(KilosortPipelineOptions {
    detect_threshold: 4.0,
    refractory_seconds: 0.001,
    min_correlation: 0.3,
    ..Default::default()
});
let result = pipeline.sort_continuous(&data, "raw")?;
result.write_phy_folder("phy_out")?;
let nex5 = pipeline.to_file_data(&result, 30_000.0, "sorted")?;

// Multichannel (same length + rate):
let multi = pipeline.sort_continuous_channels(&data, &["ch0", "ch1"])?;
```

```bash
nex5 sort rec.nex5 --continuous raw -o sorted.nex5 --threshold 4 --refractory-ms 1 --phy-dir phy_out
nex5 import-phy phy_out -o sorted.nex5 --skip-noise
```
