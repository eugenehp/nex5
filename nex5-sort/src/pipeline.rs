//! Kilosort-inspired CPU spike sorting pipeline (detect → snippets → PCA → cluster → rematch).

use nex5file::FileData;

use crate::error::{Result, SortError};

/// Configuration for [`KilosortPipeline`].
#[derive(Debug, Clone)]
pub struct KilosortPipelineOptions {
    /// Band-pass high corner (Hz).
    pub highpass_hz: f64,
    /// Band-pass low corner (Hz).
    pub lowpass_hz: f64,
    /// Detection threshold in robust std units (MAD).
    pub detect_threshold: f64,
    /// Total snippet length in samples (Kilosort default ≈ 61).
    pub snippet_samples: usize,
    /// Samples before the detection peak within each snippet.
    pub pre_samples: usize,
    /// PCA dimensions before clustering.
    pub n_pca: usize,
    /// Maximum clusters (K-means cap).
    pub max_clusters: usize,
    /// Stop adding clusters when mean waveform SNR falls below this.
    pub min_template_snr: f64,
    /// Upper bound on detected events (memory guard).
    pub max_spikes: usize,
    /// Minimum refractory interval between detections (seconds).
    pub refractory_seconds: f64,
    /// Minimum normalized correlation for template assignment (0..1).
    pub min_correlation: f64,
}

impl Default for KilosortPipelineOptions {
    fn default() -> Self {
        Self {
            highpass_hz: 300.0,
            lowpass_hz: 6_000.0,
            detect_threshold: 4.0,
            snippet_samples: 61,
            pre_samples: 20,
            n_pca: 3,
            max_clusters: 32,
            min_template_snr: 3.0,
            max_spikes: 500_000,
            refractory_seconds: 0.001,
            min_correlation: 0.3,
        }
    }
}

/// Output of a sort run.
#[derive(Debug, Clone, PartialEq)]
pub struct SortResult {
    pub sampling_rate: f64,
    /// Spike times in seconds.
    pub spike_times: Vec<f64>,
    /// Cluster id per spike (0..n_clusters-1).
    pub spike_clusters: Vec<i32>,
    /// Mean waveform per cluster `[cluster][sample]`.
    pub templates: Vec<Vec<f32>>,
}

impl SortResult {
    pub fn n_units(&self) -> usize {
        self.templates.len()
    }

    pub fn spike_times_samples(&self) -> Vec<f64> {
        self.spike_times
            .iter()
            .map(|t| t * self.sampling_rate)
            .collect()
    }

    /// Write Kilosort/Phy `spike_times.npy` + `spike_clusters.npy` to `dir`.
    pub fn write_phy_folder(&self, dir: impl AsRef<std::path::Path>) -> Result<()> {
        crate::phy::write_phy_folder(
            dir,
            &self.spike_times_samples(),
            &self.spike_clusters,
        )
    }
}

/// Kilosort-style sorter operating on multichannel traces.
#[derive(Debug, Clone, Default)]
pub struct KilosortPipeline {
    pub options: KilosortPipelineOptions,
}

impl KilosortPipeline {
    pub fn new(options: KilosortPipelineOptions) -> Self {
        Self { options }
    }

    /// Sort raw multichannel data. `channels[ch][sample]` at `sampling_rate` Hz.
    pub fn sort_traces(
        &self,
        channels: &[Vec<f32>],
        sampling_rate: f64,
    ) -> Result<SortResult> {
        if channels.is_empty() {
            return Err(SortError::Other("no channels".to_string()));
        }
        if sampling_rate <= 0.0 {
            return Err(SortError::Other("sampling_rate must be positive".to_string()));
        }
        let n_samples = channels[0].len();
        if n_samples == 0 {
            return Err(SortError::Other("empty traces".to_string()));
        }
        if !channels.iter().all(|ch| ch.len() == n_samples) {
            return Err(SortError::Other("channel length mismatch".to_string()));
        }

        let opts = &self.options;
        let filtered = preprocess(channels, sampling_rate, opts.highpass_hz, opts.lowpass_hz);
        let ref_trace = common_average_reference(&filtered);
        let noise = robust_std(&ref_trace);
        let threshold = opts.detect_threshold * noise.max(1e-6);
        let refractory = refractory_samples(opts.refractory_seconds, sampling_rate);
        let peaks = detect_peaks(&ref_trace, threshold, opts.max_spikes, refractory);
        if peaks.is_empty() {
            return Ok(SortResult {
                sampling_rate,
                spike_times: Vec::new(),
                spike_clusters: Vec::new(),
                templates: Vec::new(),
            });
        }

        let (snippets, peak_indices) =
            extract_snippets(&ref_trace, &peaks, opts.snippet_samples, opts.pre_samples);
        let pca = pca_reduce(&snippets, opts.n_pca.min(snippets[0].len()).max(1));
        let mut labels = cluster_pca(&pca, opts.max_clusters, opts.min_template_snr, &snippets);
        let templates = compute_templates(&snippets, &labels);
        refine_with_templates(&snippets, &mut labels, &templates, opts.min_correlation);
        let spike_times: Vec<f64> = peak_indices
            .iter()
            .map(|&p| p as f64 / sampling_rate)
            .collect();
        let spike_clusters = labels;

        Ok(SortResult {
            sampling_rate,
            spike_times,
            spike_clusters,
            templates,
        })
    }

    /// Sort multiple continuous variables as separate channels (same length required).
    pub fn sort_continuous_channels(
        &self,
        data: &FileData,
        channel_names: &[&str],
    ) -> Result<SortResult> {
        if channel_names.is_empty() {
            return Err(SortError::Other("no channel names".to_string()));
        }
        let mut channels = Vec::with_capacity(channel_names.len());
        let mut rate = 0.0f64;
        for name in channel_names {
            let cont = data.continuous(name)?;
            let ch_rate = cont.sampling_rate();
            if ch_rate <= 0.0 {
                return Err(SortError::Other(format!(
                    "continuous '{name}' has invalid sampling rate"
                )));
            }
            if rate == 0.0 {
                rate = ch_rate;
            } else if (rate - ch_rate).abs() > f64::EPSILON {
                return Err(SortError::Other(
                    "all channels must share the same sampling rate".to_string(),
                ));
            }
            channels.push(
                cont.continuous_values
                    .iter()
                    .map(|&v| v as f32)
                    .collect::<Vec<_>>(),
            );
        }
        let n = channels[0].len();
        if !channels.iter().all(|ch| ch.len() == n) {
            return Err(SortError::Other("channel length mismatch".to_string()));
        }
        self.sort_traces(&channels, rate)
    }

    /// Sort a single continuous variable from an on-disk session.
    pub fn sort_continuous(
        &self,
        data: &FileData,
        continuous_name: &str,
    ) -> Result<SortResult> {
        let cont = data.continuous(continuous_name)?;
        let rate = cont.sampling_rate();
        if rate <= 0.0 {
            return Err(SortError::Other(format!(
                "continuous '{continuous_name}' has invalid sampling rate"
            )));
        }
        let channel: Vec<f32> = cont
            .continuous_values
            .iter()
            .map(|&v| v as f32)
            .collect();
        self.sort_traces(&[channel], rate)
    }

    /// Write sorted units into a new `FileData` (does not mutate input).
    pub fn to_file_data(
        &self,
        result: &SortResult,
        timestamp_frequency_hz: f64,
        comment: &str,
    ) -> Result<FileData> {
        let mut out = FileData::new(timestamp_frequency_hz, comment)?;
        let mut by_cluster: std::collections::BTreeMap<i32, Vec<f64>> =
            std::collections::BTreeMap::new();
        for (&t, &c) in result.spike_times.iter().zip(result.spike_clusters.iter()) {
            by_cluster.entry(c).or_default().push(t);
        }
        for (cluster, mut times) in by_cluster {
            times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            out.add_neuron(format!("unit_{cluster}"), times, 0, cluster, 0.0, 0.0)?;
        }
        out.end_seconds = out
            .variables
            .iter()
            .map(|v| v.maximum_timestamp())
            .fold(out.beg_seconds, f64::max);
        Ok(out)
    }
}

fn preprocess(
    channels: &[Vec<f32>],
    fs: f64,
    high_hz: f64,
    low_hz: f64,
) -> Vec<Vec<f32>> {
    channels
        .iter()
        .map(|ch| bandpass(ch, fs, high_hz, low_hz))
        .collect()
}

fn bandpass(samples: &[f32], fs: f64, high_hz: f64, low_hz: f64) -> Vec<f32> {
    let hp = highpass(samples, fs, high_hz);
    lowpass(&hp, fs, low_hz)
}

fn highpass(samples: &[f32], fs: f64, cutoff_hz: f64) -> Vec<f32> {
    let rc = 1.0 / (2.0 * core::f64::consts::PI * cutoff_hz.max(1.0));
    let dt = 1.0 / fs;
    let alpha = rc / (rc + dt);
    let mut out = vec![0.0f32; samples.len()];
    if samples.is_empty() {
        return out;
    }
    out[0] = samples[0];
    for i in 1..samples.len() {
        out[i] = (alpha * (out[i - 1] as f64 + samples[i] as f64 - samples[i - 1] as f64)) as f32;
    }
    out
}

fn lowpass(samples: &[f32], fs: f64, cutoff_hz: f64) -> Vec<f32> {
    let rc = 1.0 / (2.0 * core::f64::consts::PI * cutoff_hz.max(1.0));
    let dt = 1.0 / fs;
    let alpha = dt / (rc + dt);
    let mut out = vec![0.0f32; samples.len()];
    if samples.is_empty() {
        return out;
    }
    out[0] = samples[0];
    for i in 1..samples.len() {
        out[i] = (out[i - 1] as f64 + alpha * (samples[i] as f64 - out[i - 1] as f64)) as f32;
    }
    out
}

fn refractory_samples(refractory_seconds: f64, fs: f64) -> usize {
    if refractory_seconds <= 0.0 {
        return 0;
    }
    (refractory_seconds * fs).round().max(1.0) as usize
}

fn common_average_reference(channels: &[Vec<f32>]) -> Vec<f32> {
    if channels.len() == 1 {
        return channels[0].clone();
    }
    let n = channels[0].len();
    let mut avg = vec![0.0f32; n];
    for ch in channels {
        for (i, &v) in ch.iter().enumerate() {
            avg[i] += v;
        }
    }
    let denom = channels.len() as f32;
    for v in &mut avg {
        *v /= denom;
    }
    channels[0]
        .iter()
        .zip(avg.iter())
        .map(|(&s, &m)| s - m)
        .collect()
}

fn robust_std(samples: &[f32]) -> f64 {
    const MAX_SAMPLES: usize = 50_000;
    if samples.len() <= MAX_SAMPLES {
        return robust_std_inner(samples);
    }
    let step = samples.len() / MAX_SAMPLES;
    let subsampled: Vec<f32> = samples.iter().step_by(step.max(1)).copied().collect();
    robust_std_inner(&subsampled)
}

fn robust_std_inner(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 1.0;
    }
    let mut vals: Vec<f64> = samples.iter().map(|&v| f64::from(v)).collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let med = vals[vals.len() / 2];
    let mut dev: Vec<f64> = vals.iter().map(|v| (v - med).abs()).collect();
    dev.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mad = dev[dev.len() / 2].max(1e-9);
    mad * 1.4826
}

fn detect_peaks(
    trace: &[f32],
    threshold: f64,
    max_spikes: usize,
    refractory: usize,
) -> Vec<usize> {
    let mut peaks = Vec::new();
    let mut last_peak = 0usize;
    for i in 1..trace.len().saturating_sub(1) {
        let v = trace[i].abs() as f64;
        if v > threshold
            && v >= trace[i - 1].abs() as f64
            && v >= trace[i + 1].abs() as f64
            && (peaks.is_empty() || i >= last_peak.saturating_add(refractory))
        {
            peaks.push(i);
            last_peak = i;
            if peaks.len() >= max_spikes {
                break;
            }
        }
    }
    peaks
}

fn extract_snippets(
    trace: &[f32],
    peaks: &[usize],
    snippet_len: usize,
    pre: usize,
) -> (Vec<Vec<f32>>, Vec<usize>) {
    let mut snippets = Vec::new();
    let mut indices = Vec::new();
    for &peak in peaks {
        let start = peak.saturating_sub(pre);
        let end = start + snippet_len;
        if end > trace.len() {
            continue;
        }
        snippets.push(trace[start..end].to_vec());
        indices.push(peak);
    }
    (snippets, indices)
}

fn pca_reduce(snippets: &[Vec<f32>], n_comp: usize) -> Vec<Vec<f64>> {
    if snippets.is_empty() {
        return Vec::new();
    }
    let dim = snippets[0].len();
    let n = snippets.len();
    let mut mean = vec![0.0f64; dim];
    for snip in snippets {
        for (i, &v) in snip.iter().enumerate() {
            mean[i] += f64::from(v);
        }
    }
    for m in &mut mean {
        *m /= n as f64;
    }
    let mut centered: Vec<Vec<f64>> = vec![vec![0.0; dim]; n];
    for (snip, row) in snippets.iter().zip(centered.iter_mut()) {
        for (i, &v) in snip.iter().enumerate() {
            row[i] = f64::from(v) - mean[i];
        }
    }
    // Power iteration for top components (lightweight PCA).
    let mut components = Vec::with_capacity(n_comp);
    let mut residual = centered.clone();
    for _ in 0..n_comp {
        let mut vec = vec![1.0f64; dim];
        for _ in 0..20 {
            let mut next = vec![0.0f64; dim];
            for row in &residual {
                let dot: f64 = row.iter().zip(vec.iter()).map(|(&a, &b)| a * b).sum();
                for (i, c) in next.iter_mut().enumerate() {
                    *c += dot * row[i];
                }
            }
            let norm = next.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-12);
            for v in &mut vec {
                *v /= norm;
            }
        }
        components.push(vec.clone());
        for row in &mut residual {
            let coeff: f64 = row.iter().zip(vec.iter()).map(|(&a, &b)| a * b).sum();
            for (i, r) in row.iter_mut().enumerate() {
                *r -= coeff * vec[i];
            }
        }
    }
    centered
        .into_iter()
        .map(|row| {
            components
                .iter()
                .map(|comp| row.iter().zip(comp.iter()).map(|(&a, &b)| a * b).sum())
                .collect()
        })
        .collect()
}

fn cluster_pca(
    pca: &[Vec<f64>],
    max_clusters: usize,
    min_snr: f64,
    snippets: &[Vec<f32>],
) -> Vec<i32> {
    if pca.is_empty() {
        return Vec::new();
    }
    let k = max_clusters.min(pca.len()).max(1);
    let mut labels = vec![0i32; pca.len()];
    let mut centroids = kmeans_init(pca, k);
    for _ in 0..30 {
        for (point, label) in pca.iter().zip(labels.iter_mut()) {
            let (idx, _) = centroids
                .iter()
                .enumerate()
                .map(|(i, c)| (i, l2(point, c)))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or((0, 0.0));
            *label = idx as i32;
        }
        centroids = recompute_centroids(pca, &labels, k);
    }
    prune_weak_clusters(&mut labels, snippets, min_snr);
    relabel_contiguous(&mut labels);
    labels
}

fn kmeans_init(pca: &[Vec<f64>], k: usize) -> Vec<Vec<f64>> {
    let step = pca.len() / k.max(1);
    (0..k)
        .map(|i| pca[(i * step).min(pca.len() - 1)].clone())
        .collect()
}

fn recompute_centroids(pca: &[Vec<f64>], labels: &[i32], k: usize) -> Vec<Vec<f64>> {
    let dim = pca[0].len();
    let mut sums = vec![vec![0.0f64; dim]; k];
    let mut counts = vec![0usize; k];
    for (point, &lab) in pca.iter().zip(labels.iter()) {
        let li = lab as usize;
        if li < k {
            counts[li] += 1;
            for (i, &v) in point.iter().enumerate() {
                sums[li][i] += v;
            }
        }
    }
    sums.into_iter()
        .zip(counts)
        .map(|(mut s, c)| {
            if c == 0 {
                return vec![0.0; dim];
            }
            for v in &mut s {
                *v /= c as f64;
            }
            s
        })
        .collect()
}

fn l2(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let d = x - y;
            d * d
        })
        .sum::<f64>()
        .sqrt()
}

fn prune_weak_clusters(labels: &mut [i32], snippets: &[Vec<f32>], min_snr: f64) {
    let max_lab = labels.iter().copied().max().unwrap_or(0);
    let mut keep = vec![true; max_lab as usize + 1];
    for lab in 0..=max_lab {
        keep[lab as usize] = cluster_waveform_snr(snippets, labels, lab) >= min_snr;
    }
    if keep.iter().filter(|&&k| k).count() <= 1 {
        return;
    }
    let snapshot = labels.to_vec();
    for (snip, lab) in snippets.iter().zip(labels.iter_mut()) {
        if keep[*lab as usize] {
            continue;
        }
        let best = best_template_cluster(snip, &snapshot, snippets, &keep);
        *lab = best;
    }
}

fn best_template_cluster(
    snip: &[f32],
    labels: &[i32],
    snippets: &[Vec<f32>],
    keep: &[bool],
) -> i32 {
    let max_lab = labels.iter().copied().max().unwrap_or(0);
    let mut best_lab = 0i32;
    let mut best_score = f64::NEG_INFINITY;
    for lab in 0..=max_lab {
        if !keep[lab as usize] {
            continue;
        }
        let tmpl = mean_waveform(snippets, labels, lab);
        if tmpl.is_empty() {
            continue;
        }
        let score = normalized_correlation(snip, &tmpl);
        if score > best_score {
            best_score = score;
            best_lab = lab;
        }
    }
    best_lab
}

fn mean_waveform(snippets: &[Vec<f32>], labels: &[i32], label: i32) -> Vec<f32> {
    let waves: Vec<&Vec<f32>> = snippets
        .iter()
        .zip(labels.iter())
        .filter_map(|(s, &l)| if l == label { Some(s) } else { None })
        .collect();
    if waves.is_empty() {
        return Vec::new();
    }
    let len = waves[0].len();
    let mut mean = vec![0.0f64; len];
    for w in &waves {
        for (i, &v) in w.iter().enumerate() {
            mean[i] += f64::from(v);
        }
    }
    mean.into_iter()
        .map(|v| (v / waves.len() as f64) as f32)
        .collect()
}

fn cluster_waveform_snr(snippets: &[Vec<f32>], labels: &[i32], label: i32) -> f64 {
    let mut waves: Vec<&Vec<f32>> = Vec::new();
    for (snip, &lab) in snippets.iter().zip(labels.iter()) {
        if lab == label {
            waves.push(snip);
        }
    }
    if waves.is_empty() {
        return 0.0;
    }
    let len = waves[0].len();
    let mut mean = vec![0.0f64; len];
    for w in &waves {
        for (i, &v) in w.iter().enumerate() {
            mean[i] += f64::from(v);
        }
    }
    for m in &mut mean {
        *m /= waves.len() as f64;
    }
    let peak = mean.iter().map(|v| v.abs()).fold(0.0, f64::max);
    let n_waves = waves.len();
    let mut resid = 0.0f64;
    for w in waves {
        for (i, &v) in w.iter().enumerate() {
            let d = f64::from(v) - mean[i];
            resid += d * d;
        }
    }
    let rmse = (resid / (n_waves * len) as f64).sqrt().max(1e-9);
    peak / rmse
}

fn relabel_contiguous(labels: &mut [i32]) {
    let mut map = std::collections::BTreeMap::<i32, i32>::new();
    let mut next = 0i32;
    for l in labels.iter_mut() {
        let entry = map.entry(*l).or_insert_with(|| {
            let id = next;
            next += 1;
            id
        });
        *l = *entry;
    }
}

fn compute_templates(snippets: &[Vec<f32>], labels: &[i32]) -> Vec<Vec<f32>> {
    if snippets.is_empty() {
        return Vec::new();
    }
    let max_lab = labels.iter().copied().max().unwrap_or(0) as usize;
    let len = snippets[0].len();
    let mut sums = vec![vec![0.0f64; len]; max_lab + 1];
    let mut counts = vec![0usize; max_lab + 1];
    for (snip, &lab) in snippets.iter().zip(labels.iter()) {
        let li = lab as usize;
        if li < sums.len() {
            counts[li] += 1;
            for (i, &v) in snip.iter().enumerate() {
                sums[li][i] += f64::from(v);
            }
        }
    }
    sums.into_iter()
        .zip(counts)
        .filter(|(_, c)| *c > 0)
        .map(|(s, c)| s.into_iter().map(|v| (v / c as f64) as f32).collect())
        .collect()
}

fn refine_with_templates(
    snippets: &[Vec<f32>],
    labels: &mut [i32],
    templates: &[Vec<f32>],
    min_correlation: f64,
) {
    if templates.is_empty() {
        return;
    }
    for (snip, lab) in snippets.iter().zip(labels.iter_mut()) {
        let (best, score) = best_template_match(snip, templates);
        if score >= min_correlation {
            *lab = best as i32;
        }
    }
}

fn best_template_match(snip: &[f32], templates: &[Vec<f32>]) -> (usize, f64) {
    templates
        .iter()
        .enumerate()
        .map(|(i, tmpl)| (i, normalized_correlation(snip, tmpl)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, 0.0))
}

fn normalized_correlation(a: &[f32], b: &[f32]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..n {
        let x = f64::from(a[i]);
        let y = f64::from(b[i]);
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na <= 1e-12 || nb <= 1e-12 {
        return 0.0;
    }
    dot / na.sqrt() / nb.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inject_spikes(base: f32, fs: f64, times_s: &[f64]) -> Vec<f32> {
        let n = (fs * 2.0) as usize;
        let mut trace = vec![base; n];
        for &t in times_s {
            let idx = (t * fs) as usize;
            if idx + 30 < n {
                for i in 0..30 {
                    trace[idx + i] += 50.0 * (-((i as f32 - 15.0).powi(2)) / 20.0).exp();
                }
            }
        }
        trace
    }

    #[test]
    fn sorts_synthetic_spikes() {
        let fs = 30_000.0;
        let ch = inject_spikes(0.0, fs, &[0.05, 0.15, 0.25, 0.35]);
        let pipeline = KilosortPipeline::new(KilosortPipelineOptions {
            detect_threshold: 2.0,
            highpass_hz: 100.0,
            ..Default::default()
        });
        let result = pipeline.sort_traces(&[ch], fs).unwrap();
        assert!(!result.spike_times.is_empty());
        assert_eq!(result.spike_times.len(), result.spike_clusters.len());
        assert!(!result.templates.is_empty());
    }
}
