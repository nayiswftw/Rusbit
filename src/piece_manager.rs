// piece_manager.rs
use std::collections::HashMap;
use sha1::{Sha1, Digest};
use std::io::{Error, ErrorKind};
use tokio::net::TcpStream;
use std::sync::Arc;

use crate::torrent::TorrentInfo;
use crate::file_io::write_piece_to_file_at_offset;
use crate::message::{send_message, Message};
use crate::piece_queue::PieceQueue;

/// Handles block requests, assembling blocks into pieces, verifying pieces,
/// and writing complete pieces to file.
pub struct PieceManager {
    pub torrent_info: TorrentInfo,
    received_blocks: HashMap<u32, Vec<u8>>,
}

impl PieceManager {
    pub fn new(torrent_info: TorrentInfo) -> Self {
        Self {
            torrent_info,
            received_blocks: HashMap::new(),
        }
    }

    /// For a given piece index, send a series of block requests.
    /// (Here we assume a 16 KiB block size.)
    pub async fn request_blocks(&self, stream: &mut TcpStream, piece_index: u32) -> Result<(), Error> {

		// Calculate the actual piece length, it if it's the last piece it may be smaller than 16 kb  
        let piece_length = self.torrent_info.piece_length as u32;
        let file_length = self.torrent_info.length as u32;

        let total_length = if (piece_index + 1) * piece_length > file_length {
            file_length - piece_index * piece_length
        } else {
            piece_length
        };

		
        let block_size = 1 << 14; // 16 KiB
        let mut offset = 0;
        while offset < total_length {
            let block_length = std::cmp::min(block_size, total_length - offset);
            send_message(
                stream,
                Message::Request {
                    index: piece_index,
                    begin: offset,
                    length: block_length,
                },
            )
            .await?;
            offset += block_size;
        }
        Ok(())
    }

    /// Handles an incoming piece message payload. If the full piece is received,
    /// verify its hash and write it to file.
    ///
    /// Returns `Ok(true)` if the piece is complete and written, or `Ok(false)` if not yet complete.
	/// And re-queues the piece
    pub async fn handle_piece(
        &mut self,
        payload: Vec<u8>,
        output_path: &str,
        piece_queue: &Arc<PieceQueue>,
        full_file: bool,
    ) -> Result<bool, Error> {
        if payload.len() < 8 {
            return Err(Error::new(ErrorKind::InvalidData, "Payload too short"));
        }
        
		let piece_index = u32::from_be_bytes(payload[0..4].try_into().map_err(|_| {
            Error::new(ErrorKind::InvalidData, "Failed to parse piece index")
        })?);

        let offset = u32::from_be_bytes(payload[4..8].try_into().map_err(|_| {
            Error::new(ErrorKind::InvalidData, "Failed to parse offset")
        })?);
		
        let block = &payload[8..];
        
        self.received_blocks
            .entry(piece_index)
            .or_default()
            .extend_from_slice(block);

        let piece_length = self.torrent_info.piece_length as u32;
        let file_length = self.torrent_info.length as u32;
        let total_piece_size = if (piece_index + 1) * piece_length > file_length {
            file_length - piece_index * piece_length
        } else {
            piece_length
        };

        let current_size = self.received_blocks.get(&piece_index).unwrap().len() as u32;
        println!(
            "Received block: piece={}, offset={}, block_length={}, current_size={}/{}",
            piece_index,
            offset,
            block.len(),
            current_size,
            total_piece_size
        );

        if current_size >= total_piece_size {
            let complete_piece = self.received_blocks.remove(&piece_index).unwrap();
            let verified = self.verify_piece(piece_index, &complete_piece);
            println!("Piece {} verified: {}", piece_index, verified);
            if verified {
                write_piece_to_file_at_offset(&complete_piece, piece_index, output_path, piece_length, full_file).await?;
                piece_queue.mark_piece_complete(piece_index).await;
                return Ok(true);
            } else {
                piece_queue.requeue_piece(piece_index).await;
                return Err(Error::new(ErrorKind::Other, "Piece verification failed"));
            }
        }
        Ok(false)
    }

    /// Verifies the SHA-1 hash of the piece against the expected hash.
    fn verify_piece(&self, piece_index: u32, piece_data: &[u8]) -> bool {
        let expected_hash = &self.torrent_info.pieces[piece_index as usize];
        let mut hasher = Sha1::new();
        hasher.update(piece_data);
        hasher.finalize().as_slice() == expected_hash
    }
}
