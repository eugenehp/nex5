//! Read variable headers without loading full payloads.

use nex5file::{OpenNexFile, Result};

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: read_headers FILE.nex5");

    let open = OpenNexFile::open_headers_only(&path)?;
    let data = open.data();

    println!("File: {path}");
    println!("Comment: {}", data.doc_comment());
    println!("Frequency: {} Hz", data.timestamp_frequency());
    println!("Variables: {}", data.variables.len());

    for name in data.neuron_names() {
        let loaded = data.is_variable_loaded(&name)?;
        println!("  neuron: {name} (loaded={loaded})");
    }
    for name in data.event_names() {
        let loaded = data.is_variable_loaded(&name)?;
        println!("  event: {name} (loaded={loaded})");
    }
    for name in data.continuous_names() {
        let loaded = data.is_variable_loaded(&name)?;
        println!("  continuous: {name} (loaded={loaded})");
    }

    Ok(())
}
