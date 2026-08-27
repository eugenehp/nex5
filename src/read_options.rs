/// Options controlling file read behavior.
#[derive(Debug, Clone)]
pub struct ReadOptions {
    /// When true, invalid JSON metadata is ignored instead of returning an error.
    pub ignore_metadata_errors: bool,
    /// Maximum bytes allowed for a single variable payload (0 = unlimited).
    pub max_payload_bytes: u64,
    /// Validate data offsets and payload sizes against file length.
    pub validate_layout: bool,
    /// Read variable payloads in on-disk order for sequential I/O (recommended for large files).
    pub sequential_io: bool,
    /// When true, decode independent variable payloads in parallel (requires `parallel` feature).
    pub parallel_decode: bool,
    /// Chunk size for streaming timestamp / continuous reads (elements per callback).
    pub stream_chunk_size: usize,
    /// Store loaded spike/event timestamps as `f32` seconds (half the memory of `f64`).
    pub compact_timestamps: bool,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            ignore_metadata_errors: false,
            max_payload_bytes: 512 * 1024 * 1024, // 512 MiB
            validate_layout: true,
            sequential_io: true,
            parallel_decode: false,
            stream_chunk_size: 4096,
            compact_timestamps: false,
        }
    }
}

impl ReadOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ignore_metadata_errors(mut self, ignore: bool) -> Self {
        self.ignore_metadata_errors = ignore;
        self
    }

    pub fn max_payload_bytes(mut self, bytes: u64) -> Self {
        self.max_payload_bytes = bytes;
        self
    }

    pub fn validate_layout(mut self, validate: bool) -> Self {
        self.validate_layout = validate;
        self
    }

    pub fn sequential_io(mut self, sequential: bool) -> Self {
        self.sequential_io = sequential;
        self
    }

    pub fn parallel_decode(mut self, parallel: bool) -> Self {
        self.parallel_decode = parallel;
        self
    }

    pub fn stream_chunk_size(mut self, size: usize) -> Self {
        self.stream_chunk_size = size.max(1);
        self
    }

    pub fn compact_timestamps(mut self, compact: bool) -> Self {
        self.compact_timestamps = compact;
        self
    }
}
