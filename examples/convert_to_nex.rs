//! Convert a `.nex5` recording to legacy `.nex` format (32-bit timestamps).

use nex5file::{read_nex5_file, Result, Writer};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).expect("usage: convert INPUT.nex5 OUTPUT.nex");
    let output = args.get(2).expect("usage: convert INPUT.nex5 OUTPUT.nex");

    let data = read_nex5_file(input)?;
    Writer::new().write_nex_file(&data, output)?;
    println!("Wrote {output}");
    Ok(())
}
