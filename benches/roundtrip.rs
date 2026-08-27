use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nex5file::{FileDataBuilder, NexFormat, OpenNexFile, Reader, Writer};
use std::path::PathBuf;

fn temp_nex5(c: &mut Criterion) {
    let data = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .event("ev", (0..10_000).map(|i| i as f64 * 0.001).collect())
        .unwrap()
        .neuron("nr", (0..5_000).map(|i| i as f64 * 0.002).collect(), 1, 1, 0.0, 0.0)
        .unwrap()
        .build()
        .unwrap();
    let bytes = Writer::new()
        .write_to_vec(&data, NexFormat::Nex5)
        .unwrap();
    c.bench_function("read_from_slice_15k_events", |b| {
        b.iter(|| {
            black_box(
                Reader::new()
                    .read_from_slice(black_box(&bytes), NexFormat::Nex5)
                    .unwrap(),
            )
        });
    });
    c.bench_function("write_to_vec_15k_events", |b| {
        b.iter(|| {
            black_box(
                Writer::new()
                    .write_to_vec(black_box(&data), NexFormat::Nex5)
                    .unwrap(),
            )
        });
    });
}

fn lazy_headers(c: &mut Criterion) {
    let dir = std::env::temp_dir().join(format!("nex5-bench-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("bench.nex5");
    let data = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .event("ev", vec![0.001; 50_000])
        .unwrap()
        .build()
        .unwrap();
    Writer::new().write_nex5_file(&data, &path).unwrap();
    c.bench_function("open_headers_only", |b| {
        b.iter(|| {
            black_box(
                OpenNexFile::open_headers_only(black_box(&path))
                    .unwrap()
                    .into_data(),
            )
        });
    });
    c.bench_function("load_one_variable", |b| {
        b.iter(|| {
            let mut open = OpenNexFile::open_headers_only(&path).unwrap();
            open.load_variables(&["ev"]).unwrap();
            black_box(open.into_data());
        });
    });
    let _ = std::fs::remove_dir_all(dir);
}

fn fixture_roundtrip(c: &mut Criterion) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.nex5");
    if !path.exists() {
        return;
    }
    c.bench_function("read_fixture_minimal", |b| {
        b.iter(|| black_box(nex5file::read_nex5_file(black_box(&path)).unwrap()));
    });
}

#[cfg(feature = "mmap")]
fn mmap_open(c: &mut Criterion) {
    let dir = std::env::temp_dir().join(format!("nex5-mmap-bench-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("mmap.nex5");
    let data = FileDataBuilder::new()
        .timestamp_frequency_hz(100_000.0)
        .unwrap()
        .event("ev", vec![0.001; 20_000])
        .unwrap()
        .build()
        .unwrap();
    Writer::new().write_nex5_file(&data, &path).unwrap();
    c.bench_function("open_mmap", |b| {
        b.iter(|| black_box(OpenNexFile::open_mmap(black_box(&path)).unwrap().into_data()));
    });
    let _ = std::fs::remove_dir_all(dir);
}

criterion_group!(benches, temp_nex5, lazy_headers, fixture_roundtrip);
#[cfg(feature = "mmap")]
criterion_group!(mmap_benches, mmap_open);
#[cfg(all(feature = "mmap"))]
criterion_main!(benches, mmap_benches);
#[cfg(not(feature = "mmap"))]
criterion_main!(benches);
