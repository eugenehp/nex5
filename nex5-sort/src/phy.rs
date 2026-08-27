//! Import Kilosort / Phy export folders into [`nex5file::FileData`].

use std::path::Path;

use nex5file::FileData;

use crate::error::{Result, SortError};
use crate::npy::{format_npy_header, read_i32_1d, read_spike_times_npy};

/// Options when importing a Phy/Kilosort results directory.
#[derive(Debug, Clone)]
pub struct PhyImportOptions {
    /// Timestamp frequency for the output `.nex5` file (Hz).
    pub timestamp_frequency_hz: f64,
    /// Sampling rate used during sorting (Hz). Spike times in `.npy` are in samples at this rate.
    pub sampling_rate: f64,
    /// Prefix for neuron variable names (`unit_0`, `unit_1`, …).
    pub unit_name_prefix: String,
    /// Session comment written into `FileData`.
    pub comment: String,
    /// When true, omit cluster id 0 (Kilosort noise cluster).
    pub skip_noise_cluster: bool,
}

impl Default for PhyImportOptions {
    fn default() -> Self {
        Self {
            timestamp_frequency_hz: 30_000.0,
            sampling_rate: 30_000.0,
            unit_name_prefix: "unit_".to_string(),
            comment: "Imported from Kilosort/Phy".to_string(),
            skip_noise_cluster: true,
        }
    }
}

/// Read `spike_times.npy` + `spike_clusters.npy` from `dir` and build neurons in `FileData`.
pub fn phy_to_file_data(dir: impl AsRef<Path>, options: &PhyImportOptions) -> Result<FileData> {
    let dir = dir.as_ref();
    let spike_times_path = dir.join("spike_times.npy");
    let spike_clusters_path = dir.join("spike_clusters.npy");
    if !spike_times_path.is_file() {
        return Err(SortError::Phy(format!(
            "missing {}",
            spike_times_path.display()
        )));
    }
    if !spike_clusters_path.is_file() {
        return Err(SortError::Phy(format!(
            "missing {}",
            spike_clusters_path.display()
        )));
    }

    let spike_times_samples = read_spike_times_npy(&spike_times_path)?;
    let spike_clusters = read_i32_1d(&spike_clusters_path)?;
    if spike_times_samples.len() != spike_clusters.len() {
        return Err(SortError::Phy(format!(
            "spike_times length {} != spike_clusters length {}",
            spike_times_samples.len(),
            spike_clusters.len()
        )));
    }
    if options.sampling_rate <= 0.0 || options.timestamp_frequency_hz <= 0.0 {
        return Err(SortError::Other(
            "sampling_rate and timestamp_frequency_hz must be positive".to_string(),
        ));
    }

    let mut clusters: std::collections::BTreeMap<i32, Vec<f64>> = std::collections::BTreeMap::new();
    for (&sample, &cluster) in spike_times_samples.iter().zip(spike_clusters.iter()) {
        if options.skip_noise_cluster && cluster == 0 {
            continue;
        }
        let seconds = sample / options.sampling_rate;
        clusters.entry(cluster).or_default().push(seconds);
    }

    let mut data = FileData::new(options.timestamp_frequency_hz, &options.comment)?;
    for (cluster_id, mut times) in clusters {
        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let name = format!("{}{}", options.unit_name_prefix, cluster_id);
        data.add_neuron(name, times, 0, cluster_id, 0.0, 0.0)?;
    }

    data.end_seconds = data
        .variables
        .iter()
        .map(|v| v.maximum_timestamp())
        .fold(data.beg_seconds, f64::max);
    Ok(data)
}

/// Write Kilosort-style `spike_times.npy` and `spike_clusters.npy` from a sort result.
pub fn write_phy_folder(
    dir: impl AsRef<Path>,
    spike_times_samples: &[f64],
    spike_clusters: &[i32],
) -> Result<()> {
    if spike_times_samples.len() != spike_clusters.len() {
        return Err(SortError::Phy(
            "spike_times and spike_clusters length mismatch".to_string(),
        ));
    }
    let dir = dir.as_ref();
    std::fs::create_dir_all(dir)?;
    write_f64_1d_npy(&dir.join("spike_times.npy"), spike_times_samples)?;
    write_i32_1d_npy(&dir.join("spike_clusters.npy"), spike_clusters)?;
    Ok(())
}

fn write_f64_1d_npy(path: &Path, values: &[f64]) -> Result<()> {
    write_typed_1d_npy(path, "<f8", values.len(), |mut out| {
        for &v in values {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    })
}

fn write_i32_1d_npy(path: &Path, values: &[i32]) -> Result<()> {
    write_typed_1d_npy(path, "<i4", values.len(), |mut out| {
        for &v in values {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    })
}

fn write_typed_1d_npy(
    path: &Path,
    descr: &str,
    len: usize,
    append_data: impl FnOnce(Vec<u8>) -> Vec<u8>,
) -> Result<()> {
    let header = format!(
        "{{'descr': '{descr}', 'fortran_order': False, 'shape': ({len},), }}"
    );
    let padded = format_npy_header(&header);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x93NUMPY\x01\x00");
    bytes.extend_from_slice(&(padded.len() as u16).to_le_bytes());
    bytes.extend_from_slice(padded.as_bytes());
    bytes = append_data(bytes);
    std::fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phy_roundtrip_folder() {
        let dir = tempfile::tempdir().unwrap();
        let times = vec![100.0, 200.0, 150.0];
        let clusters = vec![1, 1, 2];
        write_phy_folder(dir.path(), &times, &clusters).unwrap();
        let opts = PhyImportOptions {
            sampling_rate: 10_000.0,
            timestamp_frequency_hz: 10_000.0,
            ..Default::default()
        };
        let data = phy_to_file_data(dir.path(), &opts).unwrap();
        assert_eq!(data.neuron_names().len(), 2);
        let u1 = data.neuron("unit_1").unwrap();
        assert_eq!(u1.timestamps.as_f64_vec(), vec![0.01, 0.02]);
    }

    #[test]
    fn phy_skips_noise_cluster() {
        let dir = tempfile::tempdir().unwrap();
        write_phy_folder(dir.path(), &[100.0, 200.0], &[0, 1]).unwrap();
        let opts = PhyImportOptions {
            sampling_rate: 10_000.0,
            timestamp_frequency_hz: 10_000.0,
            skip_noise_cluster: true,
            ..Default::default()
        };
        let data = phy_to_file_data(dir.path(), &opts).unwrap();
        assert_eq!(data.neuron_names(), vec!["unit_1"]);
    }
}
