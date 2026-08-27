//! Extract spike timestamps from a neuron variable to a text file.

use nex5file::{read_nex5_file, Result};
use std::io::Write;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .expect("usage: extract_spikes FILE.nex5 NEURON [OUT.txt]");
    let neuron = args
        .get(2)
        .expect("usage: extract_spikes FILE.nex5 NEURON [OUT.txt]");
    let out_path = args.get(3).map(String::as_str).unwrap_or("-");

    let data = read_nex5_file(path)?;
    let nr = data.neuron(neuron)?;
    let mut out: Box<dyn Write> = if out_path == "-" {
        Box::new(std::io::stdout())
    } else {
        Box::new(std::fs::File::create(out_path)?)
    };

    for ts in nr.timestamps.iter_f64() {
        writeln!(out, "{ts}")?;
    }

    Ok(())
}
