use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use log::info;

#[derive(Debug, Clone)]
pub struct ProgressTracker {
    total_pieces: usize,
    downloaded_pieces: Arc<AtomicUsize>,
    start_time: Instant,
}

impl ProgressTracker {
    pub fn new(total_pieces: usize) -> Self {
        Self {
            total_pieces,
            downloaded_pieces: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
        }
    }

    pub fn increment(&self) {
        let downloaded = self.downloaded_pieces.fetch_add(1, Ordering::SeqCst) + 1;
        let percentage = (downloaded as f64 / self.total_pieces as f64) * 100.0;
        let elapsed = self.start_time.elapsed();
        let rate = downloaded as f64 / elapsed.as_secs_f64();

        info!(
            "Progress: {}/{} pieces ({:.1}%) - {:.1} pieces/sec",
            downloaded, self.total_pieces, percentage, rate
        );
    }

    pub fn is_complete(&self) -> bool {
        self.downloaded_pieces.load(Ordering::SeqCst) >= self.total_pieces
    }

    pub fn get_progress(&self) -> (usize, usize) {
        (
            self.downloaded_pieces.load(Ordering::SeqCst),
            self.total_pieces,
        )
    }
}