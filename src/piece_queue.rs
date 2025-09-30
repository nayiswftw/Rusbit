// piece_queue.rs
use std::collections::{HashSet, VecDeque};
use tokio::sync::Mutex;

/// Holds the available pieces as well as the pieces already in progress.
#[derive(Debug)]
pub struct PieceQueue {
    available: Mutex<VecDeque<u32>>,
    in_progress: Mutex<HashSet<u32>>,
}

impl PieceQueue {
    /// Creates a new `PieceQueue` with a list of available piece indices.
    pub fn new(available: VecDeque<u32>) -> Self {
        Self {
            available: Mutex::new(available),
            in_progress: Mutex::new(HashSet::new()),
        }
    }

    /// Returns the next piece that is not already in progress.
    ///
    /// This method locks both internal collections, pops pieces from the available
    /// queue until it finds one that isn’t marked as in progress, marks it as in progress,
    /// and returns it.
    pub async fn get_next_piece(&self) -> Option<u32> {
        // Lock both collections. (Make sure that the locking order is consistent
        // elsewhere in your code to avoid deadlocks.)
        let mut available = self.available.lock().await;
        let mut in_progress = self.in_progress.lock().await;

        while let Some(piece) = available.pop_front() {
            if !in_progress.contains(&piece) {
                in_progress.insert(piece);
                return Some(piece);
            }
        }
        None
    }

    pub async fn mark_piece_complete(&self, piece: u32) {
        let mut in_progress = self.in_progress.lock().await;
        in_progress.remove(&piece);
    }

    /// If a piece fails or needs to be retried, we requeue it
    pub async fn requeue_piece(&self, piece: u32) {
        {
            let mut in_progress = self.in_progress.lock().await;
            in_progress.remove(&piece);
        }
        let mut available = self.available.lock().await;
        available.push_back(piece);
    }
}
