use tokio::net::TcpStream;
use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;
use std::error::Error;
use std::io::{Error as IoError, ErrorKind};

use crate::torrent::TorrentInfo;
use crate::message::{
    Message, send_handshake, receive_handshake, send_message, read_message, send_extended_handshake,
};
use crate::piece_manager::PieceManager;
use crate::piece_queue::PieceQueue;
use crate::bencode::{bvalue_to_json, encode_bvalue, decode_bencode, BValue};
use crate::torrent::{get_integer, calculate_info_hash_from_struct};

/// The Peer structure now only holds connection and protocol state,
/// and it delegates piece-related work to the PieceManager.
pub struct Peer {
    pub peer_id: [u8; 20],
    pub remote_peer_id: Option<[u8; 20]>,
    pub info_hash: [u8; 20],
    pub piece_manager: Option<PieceManager>,
    pub remote_supports_extensions: bool,
}

impl Peer {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20], torrent_info: Option<TorrentInfo>) -> Self {
        let piece_manager = torrent_info.map(|info| PieceManager::new(info));
        Self {
            peer_id,
            remote_peer_id: None,
            info_hash,
            piece_manager,
            remote_supports_extensions: false, // will update after handshake.
        }
    }

    /// Connects to the remote peer and performs the handshake.
    pub async fn connect_and_handshake(
        &mut self,
        addr: &str,
        extension: bool,
    ) -> Result<TcpStream, Box<dyn Error + Send + Sync>> {
        let timeout_duration = Duration::from_secs(2);
        let mut stream = tokio::time::timeout(timeout_duration, TcpStream::connect(addr))
            .await
            .map_err(|_| IoError::new(ErrorKind::TimedOut, "Connection timed out"))??
            ;
        send_handshake(&mut stream, &self.info_hash, &self.peer_id, extension)
            .await
            .map_err(|e| e)?;
        let (remote_id, remote_supports_extensions) =
            receive_handshake(&mut stream, &self.info_hash)
                .await
                .map_err(|e| e)?;
        self.remote_peer_id = Some(remote_id);

        // If we send an extension indicator in the reserved bits and the peer handshakes,
        // we can assume they support extensions.
        self.remote_supports_extensions = remote_supports_extensions;

        Ok(stream)
    }


	pub fn get_torrent_info(&self) ->  Result<TorrentInfo, Box<dyn Error + Send + Sync>>  {
		if let Some(ref manager) = self.piece_manager {
			Ok(manager.torrent_info.clone())
		} else {
			Err(IoError::new(
				ErrorKind::Other,
				"PieceManager not available for handling piece",
			)
			.into())
		}
	}



    /// Runs the main loop to read and process messages.
    /// When the peer sends a Bitfield, we reply with Interested;
    /// when we receive an Unchoke, we ask for blocks;
    /// when we receive a Piece message, we delegate to the PieceManager.
    pub async fn run_message_loop(
        &mut self,
        mut stream: TcpStream,
        piece_index: u32,
        output_path: &str,
        in_progress: Arc<PieceQueue>,
        full_file: bool,
		send_extension: bool,
	    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            let message = read_message(&mut stream).await?;

            match message {
                Message::Bitfield => {
                    // If the remote supports extensions, perform an extended handshake.
                    if self.remote_supports_extensions {
                        send_extended_handshake(&mut stream).await?;
                        continue;
                    }

                    // After receiving bitfield, signal our interest.
                    send_message(&mut stream, Message::Interested)
                        .await
                        .map_err(|e| e)?;
                }
                Message::Unchoke => {
                    if let Some(ref mut manager) = self.piece_manager {
                        manager.request_blocks(&mut stream, piece_index)
                            .await
                            .map_err(|e| e)?;
                    } else {
                        return Err(IoError::new(
                            ErrorKind::Other,
                            "PieceManager not available for requesting blocks",
                        )
                        .into());
                    }
                }
                Message::Piece { payload } => {
                    if let Some(ref mut manager) = self.piece_manager {
                        let piece_complete = manager
                            .handle_piece(payload, output_path, &in_progress, full_file)
                            .await
                            .map_err(|e| e)?;
                        if piece_complete {
                            println!(
                                "Piece {} completely downloaded and written.",
                                piece_index
                            );
                            break;
                        }
                    } else {
                        return Err(IoError::new(
                            ErrorKind::Other,
                            "PieceManager not available for handling piece",
                        )
                        .into());
                    }
                }
                Message::ExtendedHandshake(payload) => {


					
                    let json_payload = bvalue_to_json(&payload);
                    let m = json_payload.get("m").ok_or_else(|| {
                        IoError::new(ErrorKind::InvalidData, "Missing key 'm' in extended handshake")
                    })?;

                    let ut_metadata = m.get("ut_metadata").ok_or_else(|| {
                        IoError::new(ErrorKind::InvalidData, "Missing key 'ut_metadata' in extended handshake")
                    })?;


                    // Try to extract the value as an integer.
                    let ut_metadata_int = ut_metadata.as_i64().ok_or_else(|| {
                        IoError::new(ErrorKind::InvalidData, "'ut_metadata' is not a number")
                    })?;
                    // Check that the number is within the valid range for u8.
                    if ut_metadata_int < 1 || ut_metadata_int > 255 {
                        return Err(IoError::new(
                            ErrorKind::InvalidData,
                            "'ut_metadata' value is out of range for u8",
                        )
                        .into());
                    }

                    println!("Peer Metadata Extension ID: {}", ut_metadata_int);
					if !send_extension {
						break
					}

                    let mut msg_map: HashMap<String, BValue> = HashMap::new();
                    msg_map.insert("msg_type".into(), BValue::Integer(0));
                    msg_map.insert("piece".into(), BValue::Integer(0));
                    let payload = encode_bvalue(&BValue::Dict(msg_map));

                    send_message(
                        &mut stream,
                        Message::RequestMetaData {
                            ext_msg_id: ut_metadata_int as u8,
                            payload,
                        },
                    )
                    .await
                    .map_err(|e| e)?;
                }
                Message::ReceiveMetaData { ext_msg_id: _u8, dict, payload } => {
					let root_dict = match dict {
						BValue::Dict(m) => m,
						_ => {
							return Err(
								IoError::new(ErrorKind::InvalidData, "Root of .torrent must be a dictionary")
									.into(),
							);
						}
					};


					let msg_type = get_integer(&root_dict, "msg_type")?;
                    let piece= get_integer(&root_dict, "piece")?;

					println!("msg_type: {}", msg_type);
					println!("piece: {}", piece);

					 let (_consumed, bvalue) = decode_bencode(&payload)
					 	.map_err(|e| IoError::new(ErrorKind::InvalidData, e))?;


					// Ensure the torrent_info is a dictionary.
					let torrent_dict = match bvalue {
						BValue::Dict(m) => m,
						_ => {
							return Err(
								IoError::new(ErrorKind::InvalidData, "Root of .torrent must be a dictionary")
									.into(),
							);
						}
					};
					let info: TorrentInfo =  TorrentInfo::from_bvalue(&torrent_dict)?;
					let info_hash = calculate_info_hash_from_struct(&info);

					if &info_hash != &self.info_hash {
						return Err(
							IoError::new(ErrorKind::InvalidData, "InfoHashes dont match")
								.into(),
						);
					}
					self.piece_manager = Some(PieceManager::new(info));
					break
				}
                _ => {
                    println!("Unhandled message: {:?}", message);
                }
            }
        }
        Ok(())
    }
}
