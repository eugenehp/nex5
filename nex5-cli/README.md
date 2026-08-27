# nex5-cli

Command-line toolkit for NeuroExplorer `.nex` / `.nex5` files.

```bash
cargo install nex5-cli   # binary: nex5  (v1.2)
```

```bash
nex5 info recording.nex5
nex5 export-spikes recording.nex5 unit_a -o spikes.csv
nex5 to-nwb recording.nex5 -o session.nwb
nex5 from-nwb session.nwb -o recording.nex5
nex5 psth recording.nex5 unit_a stim
nex5 sort recording.nex5 --continuous raw -o sorted.nex5
nex5 import-phy phy_out -o sorted.nex5
```

Depends on [`nex5file`](https://crates.io/crates/nex5file), [`nex5-analyze`](https://crates.io/crates/nex5-analyze), [`nex5-nwb`](https://crates.io/crates/nex5-nwb), and [`nex5-sort`](https://crates.io/crates/nex5-sort).
