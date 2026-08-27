/// Gaussian-smoothed firing rate (Hz) from binned spike counts.
pub fn smooth_firing_rate(counts: &[u64], bin_width: f64, sigma_bins: f64) -> Vec<f64> {
    if counts.is_empty() || bin_width <= 0.0 {
        return Vec::new();
    }
    let n = counts.len();
    let radius = sigma_bins.ceil() as usize * 3;
    let mut kernel = Vec::new();
    let mut kernel_sum = 0.0f64;
    for i in 0..=(radius * 2) {
        let x = (i as f64 - radius as f64) / sigma_bins.max(f64::EPSILON);
        let w = (-0.5 * x * x).exp();
        kernel.push(w);
        kernel_sum += w;
    }
    for w in &mut kernel {
        *w /= kernel_sum;
    }

    let mut smoothed = vec![0.0f64; n];
    for (i, smoothed_i) in smoothed.iter_mut().enumerate().take(n) {
        let mut acc = 0.0;
        for (k, &w) in kernel.iter().enumerate() {
            let j = i as isize + k as isize - radius as isize;
            if j >= 0 && (j as usize) < n {
                acc += counts[j as usize] as f64 * w;
            }
        }
        *smoothed_i = acc / bin_width;
    }
    smoothed
}
