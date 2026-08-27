//! Zero-copy views over memory-mapped variable payloads (`mmap` feature).

use crate::error::{NexError, Result};
use crate::file_data::FileData;
use crate::format::VariableHeader;
use crate::variables::{NexFileVarType, Variable};

/// Raw timestamp bytes for a spike/event/marker/waveform variable (no decode allocation).
#[derive(Debug, Clone, Copy)]
pub struct MmapTimestampsView<'a> {
    pub raw: &'a [u8],
    pub ts_data_type: i32,
    pub count: usize,
    pub freq_hz: f64,
}

impl<'a> MmapTimestampsView<'a> {
    pub fn iter_seconds(self) -> MmapTimestampIter<'a> {
        MmapTimestampIter {
            raw: self.raw,
            ts_data_type: self.ts_data_type,
            freq_hz: self.freq_hz,
            index: 0,
            count: self.count,
        }
    }

    pub fn get_seconds(&self, index: usize) -> Option<f64> {
        if index >= self.count {
            return None;
        }
        let item_size = if self.ts_data_type == 0 { 4 } else { 8 };
        let start = index * item_size;
        let end = start + item_size;
        let ticks = if self.ts_data_type == 0 {
            i32::from_le_bytes(self.raw.get(start..end)?.try_into().ok()?) as f64
        } else {
            i64::from_le_bytes(self.raw.get(start..end)?.try_into().ok()?) as f64
        };
        Some(ticks / self.freq_hz)
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Lazily decode timestamps from a mmap slice (one `f64` at a time).
#[derive(Debug, Clone)]
pub struct MmapTimestampIter<'a> {
    raw: &'a [u8],
    ts_data_type: i32,
    freq_hz: f64,
    index: usize,
    count: usize,
}

impl Iterator for MmapTimestampIter<'_> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let item_size = if self.ts_data_type == 0 { 4 } else { 8 };
        let start = self.index * item_size;
        let end = start + item_size;
        let ticks = if self.ts_data_type == 0 {
            i32::from_le_bytes(self.raw.get(start..end)?.try_into().ok()?) as f64
        } else {
            i64::from_le_bytes(self.raw.get(start..end)?.try_into().ok()?) as f64
        };
        self.index += 1;
        Some(ticks / self.freq_hz)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.count.saturating_sub(self.index);
        (rem, Some(rem))
    }
}

impl ExactSizeIterator for MmapTimestampIter<'_> {}

/// Zero-copy view of flat waveform sample bytes (`f32` or scaled `i16` in file).
#[derive(Debug, Clone, Copy)]
pub struct MmapWaveformSamplesView<'a> {
    pub raw: &'a [u8],
    pub cont_data_type: i32,
    pub n_points: u64,
    pub count: u64,
    pub ad_to_mv: f64,
    pub mv_offset: f64,
}

impl<'a> MmapWaveformSamplesView<'a> {
    pub fn wave_bytes(&self, index: usize) -> Option<&'a [u8]> {
        let n = self.n_points as usize;
        let item = self.bytes_per_sample();
        let start = index.checked_mul(n)?.checked_mul(item)?;
        self.raw.get(start..start + n * item)
    }

    /// Decode one waveform into millivolt `f32` values (allocates `n_points` floats).
    pub fn wave_f32(&self, index: usize) -> Option<Vec<f32>> {
        let bytes = self.wave_bytes(index)?;
        Some(decode_cont_bytes(
            bytes,
            self.cont_data_type,
            self.ad_to_mv,
            self.mv_offset,
        ))
    }

    pub fn iter_waves(self) -> MmapWaveIter<'a> {
        MmapWaveIter {
            view: self,
            index: 0,
        }
    }

    pub fn bytes_per_sample(&self) -> usize {
        if self.cont_data_type == 0 {
            2
        } else {
            4
        }
    }

    pub fn num_waves(&self) -> usize {
        self.count as usize
    }
}

/// Lazily decode individual waveforms from a mmap slice.
#[derive(Debug, Clone, Copy)]
pub struct MmapWaveIter<'a> {
    view: MmapWaveformSamplesView<'a>,
    index: usize,
}

impl Iterator for MmapWaveIter<'_> {
    type Item = Vec<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.view.num_waves() {
            return None;
        }
        let wave = self.view.wave_f32(self.index)?;
        self.index += 1;
        Some(wave)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.view.num_waves().saturating_sub(self.index);
        (rem, Some(rem))
    }
}

impl ExactSizeIterator for MmapWaveIter<'_> {}

/// Zero-copy view of continuous A/D sample bytes (after fragment metadata).
#[derive(Debug, Clone, Copy)]
pub struct MmapContinuousSamplesView<'a> {
    pub raw: &'a [u8],
    pub cont_data_type: i32,
    pub num_samples: usize,
    pub ad_to_mv: f64,
    pub mv_offset: f64,
}

impl<'a> MmapContinuousSamplesView<'a> {
    pub fn bytes_per_sample(&self) -> usize {
        if self.cont_data_type == 0 {
            2
        } else {
            4
        }
    }

    pub fn sample_f32(&self, index: usize) -> Option<f32> {
        let item = self.bytes_per_sample();
        let start = index.checked_mul(item)?;
        let end = start + item;
        decode_cont_sample(self.raw.get(start..end)?, self.cont_data_type, self.ad_to_mv, self.mv_offset)
    }

    pub fn iter_samples(self) -> MmapContinuousSampleIter<'a> {
        MmapContinuousSampleIter {
            view: self,
            index: 0,
        }
    }
}

/// Lazily decode continuous samples from a mmap slice.
#[derive(Debug, Clone, Copy)]
pub struct MmapContinuousSampleIter<'a> {
    view: MmapContinuousSamplesView<'a>,
    index: usize,
}

impl Iterator for MmapContinuousSampleIter<'_> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.view.num_samples {
            return None;
        }
        let v = self.view.sample_f32(self.index)?;
        self.index += 1;
        Some(v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.view.num_samples.saturating_sub(self.index);
        (rem, Some(rem))
    }
}

impl ExactSizeIterator for MmapContinuousSampleIter<'_> {}

/// Resolved layout for a variable payload inside a mmap'd file.
#[derive(Debug, Clone)]
pub enum MmapVariableView<'a> {
    Timestamps(MmapTimestampsView<'a>),
    Waveform {
        timestamps: MmapTimestampsView<'a>,
        samples: MmapWaveformSamplesView<'a>,
    },
    Continuous {
        fragment_timestamps: MmapTimestampsView<'a>,
        payload: &'a [u8],
        samples: MmapContinuousSamplesView<'a>,
    },
    Opaque(&'a [u8]),
}

impl MmapVariableView<'_> {
    pub fn variable_name_hint(&self, fallback: &str) -> String {
        let _ = self;
        fallback.to_string()
    }
}

/// Resolve a zero-copy payload view for `name` inside `file_bytes`.
pub fn mmap_view_for_variable<'a>(
    file_bytes: &'a [u8],
    data: &FileData,
    name: &str,
) -> Result<MmapVariableView<'a>> {
    let idx = data.index_of(name)?;
    let var = &data.variables[idx];
    let hw = var.header();
    if hw.data_offset as usize >= file_bytes.len() {
        return Err(NexError::InvalidDataOffset {
            name: name.to_string(),
            offset: hw.data_offset,
            file_size: file_bytes.len() as u64,
        });
    }
    let base = hw.data_offset as usize;

    match var {
        Variable::Neuron(_) | Variable::Event(_) | Variable::Marker(_) => {
            Ok(MmapVariableView::Timestamps(ts_view(
                file_bytes, base, hw, data.timestamp_frequency_hz, name,
            )?))
        }
        Variable::Waveform(_) => {
            let ts = ts_view(file_bytes, base, hw, data.timestamp_frequency_hz, name)?;
            let ts_bytes = hw.bytes_in_timestamp().saturating_mul(hw.count as usize);
            let sample_start = base + ts_bytes;
            let n_samples = hw.count.saturating_mul(hw.n_points_wave) as usize;
            let sample_bytes = n_samples.saturating_mul(hw.bytes_in_cont_value());
            let samples_raw = file_bytes
                .get(sample_start..sample_start + sample_bytes)
                .ok_or_else(|| NexError::InvalidDataOffset {
                    name: name.to_string(),
                    offset: sample_start as u64,
                    file_size: file_bytes.len() as u64,
                })?;
            Ok(MmapVariableView::Waveform {
                timestamps: ts,
                samples: MmapWaveformSamplesView {
                    raw: samples_raw,
                    cont_data_type: hw.cont_data_type,
                    n_points: hw.n_points_wave,
                    count: hw.count,
                    ad_to_mv: hw.ad_to_mv,
                    mv_offset: hw.mv_offset,
                },
            })
        }
        Variable::Continuous(_) => {
            let frag_count = hw.count as usize;
            let ts_bytes = hw.bytes_in_timestamp().saturating_mul(frag_count);
            let idx_bytes = hw.bytes_in_fragment_index().saturating_mul(frag_count);
            let meta_end = base + ts_bytes + idx_bytes;
            let num_samples = hw.n_points_wave as usize;
            let sample_bytes = num_samples.saturating_mul(hw.bytes_in_cont_value());
            let payload_end = meta_end + sample_bytes;
            let payload = file_bytes
                .get(base..payload_end)
                .ok_or_else(|| NexError::InvalidDataOffset {
                    name: name.to_string(),
                    offset: hw.data_offset,
                    file_size: file_bytes.len() as u64,
                })?;
            let samples_raw = payload
                .get(ts_bytes + idx_bytes..)
                .ok_or_else(|| NexError::InvalidDataOffset {
                    name: name.to_string(),
                    offset: meta_end as u64,
                    file_size: file_bytes.len() as u64,
                })?;
            Ok(MmapVariableView::Continuous {
                fragment_timestamps: ts_view(
                    file_bytes,
                    base,
                    hw,
                    data.timestamp_frequency_hz,
                    name,
                )?,
                payload,
                samples: MmapContinuousSamplesView {
                    raw: samples_raw,
                    cont_data_type: hw.cont_data_type,
                    num_samples,
                    ad_to_mv: hw.ad_to_mv,
                    mv_offset: hw.mv_offset,
                },
            })
        }
        _ => {
            let end = file_bytes.len().min(base + payload_size_estimate(hw));
            Ok(MmapVariableView::Opaque(
                file_bytes
                    .get(base..end)
                    .ok_or_else(|| NexError::InvalidDataOffset {
                        name: name.to_string(),
                        offset: hw.data_offset,
                        file_size: file_bytes.len() as u64,
                    })?,
            ))
        }
    }
}

fn ts_view<'a>(
    file_bytes: &'a [u8],
    base: usize,
    hw: &VariableHeader,
    freq_hz: f64,
    name: &str,
) -> Result<MmapTimestampsView<'a>> {
    let ts_bytes = hw.bytes_in_timestamp().saturating_mul(hw.count as usize);
    let raw = file_bytes
        .get(base..base + ts_bytes)
        .ok_or_else(|| NexError::InvalidDataOffset {
            name: name.to_string(),
            offset: hw.data_offset,
            file_size: file_bytes.len() as u64,
        })?;
    Ok(MmapTimestampsView {
        raw,
        ts_data_type: hw.ts_data_type,
        count: hw.count as usize,
        freq_hz,
    })
}

fn decode_cont_bytes(raw: &[u8], cont_data_type: i32, ad_to_mv: f64, mv_offset: f64) -> Vec<f32> {
    if cont_data_type == 0 {
        raw.chunks_exact(2)
            .map(|chunk| {
                decode_cont_sample(chunk, cont_data_type, ad_to_mv, mv_offset).unwrap_or(0.0)
            })
            .collect()
    } else {
        raw.chunks_exact(4)
            .map(|chunk| {
                decode_cont_sample(chunk, cont_data_type, ad_to_mv, mv_offset).unwrap_or(0.0)
            })
            .collect()
    }
}

fn decode_cont_sample(
    bytes: &[u8],
    cont_data_type: i32,
    ad_to_mv: f64,
    mv_offset: f64,
) -> Option<f32> {
    if cont_data_type == 0 {
        let raw = i16::from_le_bytes(bytes.try_into().ok()?);
        let scale = if ad_to_mv > 0.0 { ad_to_mv } else { 1.0 };
        Some((f64::from(raw) * scale + mv_offset) as f32)
    } else {
        Some(f32::from_le_bytes(bytes.try_into().ok()?))
    }
}

fn payload_size_estimate(hw: &VariableHeader) -> usize {
    match NexFileVarType::from_i32(hw.var_type) {
        Some(NexFileVarType::Interval) => hw.bytes_in_timestamp() * hw.count as usize * 2,
        Some(NexFileVarType::PopulationVector) => 8 * hw.count as usize,
        _ => hw.bytes_in_timestamp() * hw.count as usize,
    }
}
