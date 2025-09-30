use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;

#[derive(Debug, Clone)]
pub struct ProgressTracker {
    total_pieces: usize,
    downloaded_pieces: Arc<AtomicUsize>,
    start_time: Instant,
    progress_bar: Option<ProgressBar>,
}

impl ProgressTracker {
    pub fn new(total_pieces: usize) -> Self {
        Self {
            total_pieces,
            downloaded_pieces: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
            progress_bar: None,
        }
    }

    pub fn with_progress_bar(total_pieces: usize, show_progress: bool) -> Self {
        let progress_bar = if show_progress {
            let pb = ProgressBar::new(total_pieces as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                    .unwrap()
                    .progress_chars("=> ")
            );
            pb.set_message("Downloading pieces...");
            Some(pb)
        } else {
            None
        };

        Self {
            total_pieces,
            downloaded_pieces: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
            progress_bar,
        }
    }

    pub fn increment(&self) {
        let downloaded = self.downloaded_pieces.fetch_add(1, Ordering::SeqCst) + 1;
        let percentage = (downloaded as f64 / self.total_pieces as f64) * 100.0;
        let elapsed = self.start_time.elapsed();
        let rate = downloaded as f64 / elapsed.as_secs_f64();

        if let Some(pb) = &self.progress_bar {
            pb.set_position(downloaded as u64);
            pb.set_message(format!("{:.1} pieces/sec", rate));
        } else {
            info!(
                "Progress: {}/{} pieces ({:.1}%) - {:.1} pieces/sec",
                downloaded, self.total_pieces, percentage, rate
            );
        }
    }

    pub fn finish(&self) {
        if let Some(pb) = &self.progress_bar {
            pb.finish_with_message("Download complete!");
        } else {
            let elapsed = self.start_time.elapsed();
            info!("Download completed in {:.2}s", elapsed.as_secs_f64());
        }
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

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        // Don't clear the progress bar - let finish_with_message handle it
    }
}