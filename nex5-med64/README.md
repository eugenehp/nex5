# nex5-med64

Convert MED64 `.modat` spike detection output into [`nex5file`](../) sessions.

Depends on [`med64`](https://crates.io/crates/med64) `0.0.2` and [`nex5file`](https://crates.io/crates/nex5file) `1.2` from crates.io.

```toml
[dependencies]
nex5-med64 = "1.2"
```

```rust
use nex5_med64::{modat_to_file_data, Med64ConvertOptions};

let data = modat_to_file_data("recording.modat", &Med64ConvertOptions::default())?;
```

Optional integration tests look for sample `.modat` files under a sibling `../med64/data/` checkout; they skip when absent.
