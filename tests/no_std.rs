//! Ensures the crate builds and links in `#![no_std]` mode.

#![no_std]

extern crate alloc;

use alloc::vec;
use nex5file::{FileData, NexFormat, Reader, Writer};

#[test]
fn no_std_slice_roundtrip() {
    let mut data = FileData::new(100_000.0, "no_std test").expect("valid file data");
    data.add_event("spikes", vec![0.001, 0.002, 0.003])
        .expect("add event");

    let bytes = Writer::new()
        .write_to_vec(&data, NexFormat::Nex5)
        .expect("write to vec");

    let back = Reader::new()
        .read_from_slice(&bytes, NexFormat::Nex5)
        .expect("read from slice");

    assert_eq!(back.event_names(), data.event_names());
    assert_eq!(
        back.event("spikes").expect("event").timestamps,
        data.event("spikes").expect("event").timestamps
    );
}
