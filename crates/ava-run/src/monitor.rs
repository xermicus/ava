//! Watching the agent output for signs of a stuck run.

/// A line shorter than this is too generic to count as a repeat.
const MINIMUM_LINE_BYTES: usize = 16;

/// Consecutive repeats of one line before the run counts as looping.
pub(crate) const REPEATED_LINE_THRESHOLD: u32 = 10;

/// What the output reader threads report and the run loop reads.
struct Inner {
    output_bytes: u64,
    last_output: std::time::Instant,
    /// The current line, hashed as it streams so no line is buffered.
    line_hasher: std::collections::hash_map::DefaultHasher,
    line_bytes: usize,
    last_line: Option<u64>,
    repeats: u32,
    doom_looping: bool,
}

/// The agent output statistics shared between the reader threads and the run
/// loop.
pub(crate) struct Monitor {
    inner: std::sync::Mutex<Inner>,
}

impl Monitor {
    pub(crate) fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(Inner {
                output_bytes: 0,
                last_output: std::time::Instant::now(),
                line_hasher: Default::default(),
                line_bytes: 0,
                last_line: None,
                repeats: 0,
                doom_looping: false,
            }),
        }
    }

    /// Start watching a new sandbox on the counters of the run.
    ///
    /// The bytes are the console of the whole run, so they carry over, while
    /// the silence clock and the repeat detector are about the process that is
    /// live and start over with it.
    pub(crate) fn restart(&self) {
        let mut inner = self.lock();
        inner.last_output = std::time::Instant::now();
        inner.line_hasher = Default::default();
        inner.line_bytes = 0;
        inner.last_line = None;
        inner.repeats = 0;
        inner.doom_looping = false;
    }

    /// Count `chunk` and, on the line scanned stream, watch for repeats.
    pub(crate) fn observe(&self, chunk: &[u8], scan_lines: bool) {
        let mut inner = self.lock();
        inner.output_bytes += chunk.len() as u64;
        inner.last_output = std::time::Instant::now();

        if !scan_lines {
            return;
        }

        let mut rest = chunk;
        while let Some(position) = rest.iter().position(|byte| *byte == b'\n') {
            let (tail, remainder) = rest.split_at(position);
            inner.extend_line(tail);
            inner.complete_line();
            rest = &remainder[1..];
        }
        inner.extend_line(rest);
    }

    /// The bytes the agent printed so far.
    pub(crate) fn output_bytes(&self) -> u64 {
        self.lock().output_bytes
    }

    /// How long the agent has printed nothing.
    pub(crate) fn silent_for(&self) -> std::time::Duration {
        self.lock().last_output.elapsed()
    }

    /// Whether one line repeated often enough to look like a loop.
    pub(crate) fn doom_looping(&self) -> bool {
        self.lock().doom_looping
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("the monitor is not poisoned")
    }
}

impl Inner {
    fn extend_line(&mut self, bytes: &[u8]) {
        std::hash::Hasher::write(&mut self.line_hasher, bytes);
        self.line_bytes += bytes.len();
    }

    /// Compare the completed line against the one before it.
    fn complete_line(&mut self) {
        let hash = std::hash::Hasher::finish(&self.line_hasher);
        self.line_hasher = Default::default();
        let bytes = std::mem::take(&mut self.line_bytes);

        if bytes < MINIMUM_LINE_BYTES {
            self.last_line = None;
            self.repeats = 0;
            return;
        }

        if self.last_line == Some(hash) {
            self.repeats += 1;
            self.doom_looping |= self.repeats >= REPEATED_LINE_THRESHOLD;
        } else {
            self.last_line = Some(hash);
            self.repeats = 1;
        }
    }
}
