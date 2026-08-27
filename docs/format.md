# NeuroExplorer .nex / .nex5 binary format

This document describes the on-disk layout implemented by `nex5file`. Offsets are little-endian.

## File header

### `.nex` (544 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0 | i32 | Magic `827868494` |
| 4 | i32 | Version (106) |
| 8 | char[256] | Comment |
| 264 | f64 | Timestamp frequency (Hz) |
| 272 | i32 | Begin ticks |
| 276 | i32 | End ticks |
| 280 | i32 | Number of variables |
| 284 | char[260] | Padding |

### `.nex5` (356 bytes)

| Offset | Type | Field |
|--------|------|-------|
| 0 | i32 | Magic `894977358` |
| 4 | i32 | Version (501 or 502 for 64-bit timestamps) |
| 8 | char[256] | Comment |
| 264 | f64 | Timestamp frequency (Hz) |
| 272 | i64 | Begin ticks |
| 280 | i32 | Number of variables |
| 284 | u64 | Metadata JSON offset (0 if none) |
| 292 | i64 | End ticks |
| 300 | char[56] | Padding |

Metadata is UTF-8 JSON appended after variable data; the offset at byte 284 is patched after writing.

## Variable header

### `.nex` (208 bytes each)

Includes type, name (64 bytes), data offset (i32), count (i32), wire/unit/gain/filter, x/y position, sampling rate, A/D scaling, waveform points, marker fields, etc.

### `.nex5` (244 bytes each)

Same logical fields with u64/i64 data offset and count, timestamp data type (0=i32, 1=i64), continuous data type (0=i16, 1=f32), fragment index type, and 32-byte units string.

Variable headers are stored contiguously after the file header. Payload begins at the first variable's `data_offset`.

## Variable types

| Type ID | Name |
|---------|------|
| 0 | Neuron |
| 1 | Event |
| 2 | Interval |
| 3 | Waveform |
| 4 | Population vector |
| 5 | Continuous |
| 6 | Marker |

### Payload layout

- **Event / Neuron**: `count` timestamps
- **Interval**: `count` start timestamps + `count` end timestamps
- **Marker**: timestamps, then per field: 64-byte name + `count` values (fixed-length strings or u32)
- **Continuous**: fragment timestamps, u32 fragment indexes, continuous samples
- **Waveform**: timestamps, then `count × n_points` samples
- **Population vector**: `count` f64 weights

Timestamps are stored as integer ticks; seconds = ticks / file frequency.

## Limitations

- **64-bit fragment indexes**: not supported for writing (same as Python `nex5file`; only u32 indexes are emitted).
- **Empty files**: this library writes truly empty files. NeuroExplorer may add `StartStop` / `AllFile` variables when opening empty files in the GUI.

## Reference

- Python reference: [NeuroExplorer/nex5file](https://github.com/NeuroExplorer/nex5file)
- Official docs: [neuroexplorer.com/nex5file](https://www.neuroexplorer.com/nex5file/)
