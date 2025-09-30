// src/engine.rs
use reqwest::Client;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;

use crate::bencode::{decode_bencode, bvalue_to_json};
use crate::magnet::decode_magnet;
use crate::torrent::Torrent;
use crate::peer::Peer;
use crate::tracker;
use crate::utils;
use crate::piece_queue::PieceQueue;

#[tokio::main]
pub async fn use_command(args: Vec<String>) -> Result<(), Box<dyn Error + Send + Sync + Send + Sync>> {
    // Expect at least one command.
    if args.len() < 2 {
        eprintln!("No command provided.");
        return Ok(());
    }
    let command = &args[1];

    match command.as_str() {
        "decode" => {
            if args.len() < 3 {
                eprintln!("Usage: decode <bencoded_string>");
                return Ok(());
            }
            match decode_bencode(args[2].as_bytes()) {
                Ok((_consumed, value)) => {
                    let json_val = bvalue_to_json(&value);
                    println!("{}", serde_json::to_string(&json_val)?);
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        }
        "info" => {
            if args.len() < 3 {
                eprintln!("Usage: info <torrent_file>");
                return Ok(());
            }
            match Torrent::from_file(&args[2]) {
                Ok(torrent) => {
                    println!("Info Hash: {}", hex::encode(torrent.info_hash));
                    println!("Tracker URL: {}", torrent.announce);
                    println!("File Name: {}", torrent.info.name);
                    println!("Length: {}", torrent.info.length);
                    println!("Piece Length: {}", torrent.info.piece_length);
                    println!("Number of Pieces: {}", torrent.info.pieces.len());

                    for piece_hash in &torrent.info.pieces {
                        println!("{}", hex::encode(piece_hash));
                    }
                }
                Err(err) => {
                    eprintln!("Error reading torrent: {:?}", err);
                }
            }
        }
        "peers" => {
            if args.len() < 3 {
                eprintln!("Usage: peers <torrent_file>");
                return Ok(());
            }
            let http_client = Client::new();
            let peer_id = utils::generate_peer_id();
            let uploaded = 0u64;
            let downloaded = 0u64;
            let port = 6881;

            let torrent = Torrent::from_file(&args[2])?;
            let potential_peers = tracker::announce(
                &http_client,
                &torrent.announce,
                &torrent.info_hash,
                &peer_id,
                uploaded,
                downloaded,
                torrent.info.length as u64,
                port,
            )
            .await?;

            for (ip, port) in potential_peers {
                let addr = format!("{}:{}", ip, port);
                println!("{}", addr);
            }
        }
        "handshake" => {
            if args.len() < 4 {
                eprintln!("Usage: handshake <torrent_file> <peer_addr>");
                return Ok(());
            }
            // setup_peer returns (Peer, TcpStream) on success.
            match setup_peer(&args[2], &args[3]).await {
                Ok((_peer, _stream)) => println!("Handshake successful with peer"),
                Err(e) => eprintln!("Error during handshake: {}", e),
            }
        }
        "download_piece" => {
            if args.len() < 6 || args[2] != "-o" {
                eprintln!("Usage: download_piece -o <output_path> <torrent_file> <piece_index>");
                return Ok(());
            }
            let output_path = &args[3];
            let torrent_path = &args[4];
            let piece_index: u32 = args[5].parse().expect("Invalid piece index");

            let http_client = Client::new();
            let peer_id = utils::generate_peer_id();
            let port = 6881;
            let uploaded = 0;
            let downloaded = 0;

            let torrent = Torrent::from_file(torrent_path)?;
            let potential_peers = tracker::announce(
                &http_client,
                &torrent.announce,
                &torrent.info_hash,
                &peer_id,
                uploaded,
                downloaded,
                torrent.info.length as u64,
                port,
            )
            .await?;

            if potential_peers.is_empty() {
                eprintln!("No Peers Available");
                return Ok(());
            }

            let (ip, port) = &potential_peers[0];
            let addr = format!("{}:{}", ip, port);

            // Create a piece queue containing only the one piece we want.
            let piece_queue = Arc::new(PieceQueue::new(VecDeque::from(vec![piece_index])));

            let torrent_path = torrent_path.to_string();
            let output_path = output_path.to_string();
            let addr_clone = addr.clone();
            let pq_clone = Arc::clone(&piece_queue);

            let handle = tokio::spawn(async move {
                while let Some(piece) = pq_clone.get_next_piece().await {
                    println!("Peer {} downloading piece {}", addr_clone, piece);

                    match setup_peer(&torrent_path, &addr_clone).await {
                        Ok((mut peer, stream)) => {
                            if let Err(e) = peer
                                .run_message_loop(stream, piece, &output_path, Arc::clone(&pq_clone), false, false)
                                .await
                            {
                                eprintln!("Error processing messages for {}: {}", addr_clone, e);
                            }
                        }
                        Err(e) => eprintln!("Failed to setup peer {}: {}", addr_clone, e),
                    }
                }
            });
            handle.await?;
        }
        "download" => {
            if args.len() < 5 || args[2] != "-o" {
                eprintln!("Usage: download -o <output_path> <torrent_file>");
                return Ok(());
            }

            let output_path = &args[3];
            let torrent_path = &args[4];

            let http_client = Client::new();
            let peer_id = utils::generate_peer_id();

            let torrent = Torrent::from_file(torrent_path)?;
            let potential_peers = tracker::announce(
                &http_client,
                &torrent.announce,
                &torrent.info_hash,
                &peer_id,
                0,
                0,
                torrent.info.length as u64,
                6881,
            )
            .await?;

            // Create a piece queue containing all piece indices.
            let pieces: Vec<u32> = (0..torrent.info.pieces.len()).map(|i| i as u32).collect();
            let piece_queue = Arc::new(PieceQueue::new(VecDeque::from(pieces)));

            let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
            for (ip, port) in potential_peers {
                let addr = format!("{}:{}", ip, port);
                let torrent_path = torrent_path.to_string();
                let output_path = output_path.to_string();
                let pq = Arc::clone(&piece_queue);

                let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                    while let Some(piece) = pq.get_next_piece().await {
                        println!("Peer {} downloading piece {}", addr, piece);

                        match setup_peer(&torrent_path, &addr).await {
                            Ok((mut peer, stream)) => {
                                if let Err(e) = peer
                                    .run_message_loop(stream, piece, &output_path, Arc::clone(&pq), true, false)
                                    .await
                                {
                                    eprintln!("Error processing messages for {}: {}", addr, e);
                                }
                            }
                            Err(e) => eprintln!("Failed to setup peer {}: {}", addr, e),
                        }
                    }
                });
                handles.push(handle);
            }
            for handle in handles {
                handle.await?;
            }
        }
        "magnet_parse" => {
            if args.len() < 3 {
                eprintln!("Usage: magnet_parse <magnet_link>");
                return Ok(());
            }
            let magnet_map = decode_magnet(&args[2])?;
            let info_hash = magnet_map.get("info_hash").unwrap();
            let announce = magnet_map.get("announce").unwrap();
            println!("Tracker URL: {}", announce);
            println!("Info Hash: {}", info_hash);
        }
		"magnet_handshake" => {
			if args.len() < 3 {
				eprintln!("Usage: magnet_handshake <magnet_link>");
				return Ok(());
			}
			// Parse magnet link.
			let magnet_map = decode_magnet(&args[2])?;
			let info_hash = magnet_map.get("info_hash").unwrap();
			let announce = magnet_map.get("announce").unwrap();
			println!("Tracker URL: {}", announce);
			let http_client = Client::new();
			let peer_id = utils::generate_peer_id();
	
			let info_hash_bytes: [u8; 20] = {
				let mut bytes = [0u8; 20];
				hex::decode_to_slice(info_hash, &mut bytes)
					.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
				bytes
			};
	
			// Announce to tracker to get a list of potential peers.
			let potential_peers = tracker::announce(
				&http_client,
				announce,
				&info_hash_bytes,
				&peer_id,
				0,
				0,
				10,
				6881,
			)
			.await?;
	
			// For metadata retrieval, connect to the first available peer.
	
			let (ip, port) = potential_peers
				.first()
				.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No peers available"))?;
			let addr = format!("{}:{}", ip, port);
			println!("Using peer {} for metadata", addr);
	
			// Create a temporary peer instance to fetch metadata.
			let mut meta_peer = Peer::new(info_hash_bytes, utils::generate_peer_id(), None);
			let stream = meta_peer.connect_and_handshake(&addr, true).await?;
	
			meta_peer.run_message_loop(
				stream,
				0,
				"test.rs",
				Arc::new(PieceQueue::new(VecDeque::new())),
				false, 
				false          
			).await?;

			if let Some(remote_id) = meta_peer.remote_peer_id {
				println!("Peer ID: {}", hex::encode(remote_id));
			} else {
				println!("Failed to fetch remote id");
			}	
		}
		"magnet_info" => {
			if args.len() < 3 {
				eprintln!("Usage: magnet_handshake <magnet_link>");
				return Ok(());
			}
			// Parse magnet link.
			let magnet_map = decode_magnet(&args[2])?;
			let info_hash = magnet_map.get("info_hash").unwrap();
			let announce = magnet_map.get("announce").unwrap();
			println!("Tracker URL: {}", announce);
			let http_client = Client::new();
			let peer_id = utils::generate_peer_id();

			let info_hash_bytes: [u8; 20] = {
				let mut bytes = [0u8; 20];
				hex::decode_to_slice(info_hash, &mut bytes)
					.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
				bytes
			};

			// Announce to tracker to get a list of potential peers.
			let potential_peers = tracker::announce(
				&http_client,
				announce,
				&info_hash_bytes,
				&peer_id,
				0,
				0,
				10,
				6881,
			)
			.await?;

			let (ip, port) = potential_peers
				.first()
				.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No peers available"))?;
			let addr = format!("{}:{}", ip, port);
			println!("Using peer {} for metadata", addr);

			// Create a temporary peer instance to fetch metadata.
			let mut meta_peer = Peer::new(info_hash_bytes, utils::generate_peer_id(), None);
			let stream = meta_peer.connect_and_handshake(&addr, true).await?;

			meta_peer.run_message_loop(
				stream,
				0,
				"test.rs",
				Arc::new(PieceQueue::new(VecDeque::new())),
				false, 
				true                   
			).await?;

			if let Some(remote_id) = meta_peer.remote_peer_id {
				println!("Peer ID: {}", hex::encode(remote_id));
			} else {
				println!("Failed to fetch remote id");
			}

			// Retrieve the torrent metadata.
			let info = meta_peer.get_torrent_info()?;
			println!("Info Hash: {}", hex::encode(meta_peer.info_hash));
			println!("Tracker URL: {}", announce);
			println!("File Name: {}", info.name);
			println!("Length: {}", info.length);
			println!("Piece Length: {}", info.piece_length);
			println!("Number of Pieces: {}", info.pieces.len());
			for piece_hash in &info.pieces {
				println!("{}", hex::encode(piece_hash));
			}
        }

		"magnet_download_piece" => {

			// Convert these arguments into owned Strings.
			let output_path = args[3].to_string();
			let magnet_link = args[4].to_string();
			let piece_index: u32 = args[5].parse().expect("Invalid piece index");

			// Use the owned magnet_link for decoding.
			let magnet_map = decode_magnet(&magnet_link)?;
			let info_hash = magnet_map.get("info_hash").unwrap();
			let announce = magnet_map.get("announce").unwrap();
			println!("Tracker URL: {}", announce);

			let http_client = Client::new();
			let peer_id = utils::generate_peer_id();

			let info_hash_bytes: [u8; 20] = {
				let mut bytes = [0u8; 20];
				hex::decode_to_slice(info_hash, &mut bytes)
					.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
				bytes
			};

			// Announce to tracker to get a list of potential peers.
			let potential_peers = tracker::announce(
				&http_client,
				announce,
				&info_hash_bytes,
				&peer_id,
				0,
				0,
				10,
				6881,
			)
			.await?;

			let (ip, port) = potential_peers
				.first()
				.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No peers available"))?;
			let addr = format!("{}:{}", ip, port);
			println!("Using peer {} for metadata", addr);

			// Create a temporary peer instance to fetch metadata.
			let mut meta_peer = Peer::new(info_hash_bytes, utils::generate_peer_id(), None);
			let stream = meta_peer.connect_and_handshake(&addr, true).await?;

			meta_peer
				.run_message_loop(
					stream,
					0,
					"test.rs",
					Arc::new(PieceQueue::new(VecDeque::new())),
					false, 
					true,
				)
				.await?;

			if let Some(remote_id) = meta_peer.remote_peer_id {
				println!("Peer ID: {}", hex::encode(remote_id));
			} else {
				println!("Failed to fetch remote id");
			}

			// Retrieve the torrent metadata.
			let info = meta_peer.get_torrent_info()?;
			println!("Info Hash: {}", hex::encode(meta_peer.info_hash));
			println!("Tracker URL: {}", announce);
			println!("File Name: {}", info.name);
			println!("Length: {}", info.length);
			println!("Piece Length: {}", info.piece_length);
			println!("Number of Pieces: {}", info.pieces.len());
			for piece_hash in &info.pieces {
				println!("{}", hex::encode(piece_hash));
			}

			// Create a piece queue containing only the target piece.
			let full_piece_queue = Arc::new(PieceQueue::new(VecDeque::from(vec![piece_index])));

			// Now spawn download tasks for each available peer.
			let mut handles = Vec::new();
			for (ip, port) in potential_peers {
				let addr = format!("{}:{}", ip, port);
				// Clone the owned data for use in the async task.
				let info_clone = info.clone();
				let piece_queue_clone = Arc::clone(&full_piece_queue);
				let info_hash_bytes_clone = info_hash_bytes;
				let addr_clone = addr.clone();
				let output_path_clone = output_path.clone();

				let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
					// For each piece, create a new connection.
					while let Some(piece) = piece_queue_clone.get_next_piece().await {
						println!("Peer {} downloading piece {}", addr_clone, piece);

						// Establish a fresh connection for each piece.
						match Peer::new(info_hash_bytes_clone, utils::generate_peer_id(), Some(info_clone.clone()))
							.connect_and_handshake(&addr_clone, false)
							.await
						{
							Ok(stream) => {
								// Create a new peer instance for this connection.
								let mut download_peer = Peer::new(
									info_hash_bytes_clone,
									utils::generate_peer_id(),
									Some(info_clone.clone()),
								);

								if let Err(e) = download_peer
									.run_message_loop(
										stream,
										piece,
										&output_path_clone,
										Arc::clone(&piece_queue_clone),
										false,
										false,
									)
									.await
								{
									eprintln!("Error processing messages for {}: {}", addr_clone, e);
								}
							}
							Err(e) => {
								eprintln!("Failed to setup peer {}: {}", addr_clone, e);
							}
						}
					}
				});

				handles.push(handle);
			}

			// Wait for all download tasks to complete.
			for handle in handles {
				handle.await?;
			}
		}
		"magnet_download" => {
            let output_path = &args[3];
			
			// Parse magnet link.
			let magnet_map = decode_magnet(&args[4])?;
			let info_hash = magnet_map.get("info_hash").unwrap();
			let announce = magnet_map.get("announce").unwrap();
			println!("Tracker URL: {}", announce);
			let http_client = Client::new();
			let peer_id = utils::generate_peer_id();

			let info_hash_bytes: [u8; 20] = {
				let mut bytes = [0u8; 20];
				hex::decode_to_slice(info_hash, &mut bytes)
					.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
				bytes
			};

			// Announce to tracker to get a list of potential peers.
			let potential_peers = tracker::announce(
				&http_client,
				announce,
				&info_hash_bytes,
				&peer_id,
				0,
				0,
				10,
				6881,
			)
			.await?;

			let (ip, port) = potential_peers
				.first()
				.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No peers available"))?;
			let addr = format!("{}:{}", ip, port);
			println!("Using peer {} for metadata", addr);

			// Create a temporary peer instance to fetch metadata.
			let mut meta_peer = Peer::new(info_hash_bytes, utils::generate_peer_id(), None);
			let stream = meta_peer.connect_and_handshake(&addr, true).await?;

			meta_peer.run_message_loop(
				stream,
				0,
				"test.rs",
				Arc::new(PieceQueue::new(VecDeque::new())),
				false, 
				true                   
			).await?;

			if let Some(remote_id) = meta_peer.remote_peer_id {
				println!("Peer ID: {}", hex::encode(remote_id));
			} else {
				println!("Failed to fetch remote id");
			}

			// Retrieve the torrent metadata.
			let info = meta_peer.get_torrent_info()?;
			println!("Info Hash: {}", hex::encode(meta_peer.info_hash));
			println!("Tracker URL: {}", announce);
			println!("File Name: {}", info.name);
			println!("Length: {}", info.length);
			println!("Piece Length: {}", info.piece_length);
			println!("Number of Pieces: {}", info.pieces.len());
			for piece_hash in &info.pieces {
				println!("{}", hex::encode(piece_hash));
			}

			// Build a full piece queue from all piece indices.
			let num_pieces = info.pieces.len();
			let pieces: Vec<u32> = (0..num_pieces).map(|i| i as u32).collect();
			let full_piece_queue = Arc::new(PieceQueue::new(VecDeque::from(pieces)));

			// Now spawn download tasks for each available peer.
			let mut handles = Vec::new();
			for (ip, port) in potential_peers {
				let addr = format!("{}:{}", ip, port);
				// Clone the owned data for use in the async task.
				let info_clone = info.clone();
				let piece_queue_clone = Arc::clone(&full_piece_queue);
				let info_hash_bytes_clone = info_hash_bytes;
				let addr_clone = addr.clone();
				let output_path_clone = output_path.clone();

				let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
					// For each piece, create a new connection.
					while let Some(piece) = piece_queue_clone.get_next_piece().await {
						println!("Peer {} downloading piece {}", addr_clone, piece);

						// Establish a fresh connection for each piece.
						match Peer::new(info_hash_bytes_clone, utils::generate_peer_id(), Some(info_clone.clone()))
							.connect_and_handshake(&addr_clone, false)
							.await
						{
							Ok(stream) => {
								// Create a new peer instance for this connection.
								let mut download_peer = Peer::new(
									info_hash_bytes_clone,
									utils::generate_peer_id(),
									Some(info_clone.clone()),
								);

								if let Err(e) = download_peer
									.run_message_loop(
										stream,
										piece,
										&output_path_clone,
										Arc::clone(&piece_queue_clone),
										true,
										false,
									)
									.await
								{
									eprintln!("Error processing messages for {}: {}", addr_clone, e);
								}
							}
							Err(e) => {
								eprintln!("Failed to setup peer {}: {}", addr_clone, e);
							}
						}
					}
				});

				handles.push(handle);
			}

			// Wait for all download tasks to complete.
			for handle in handles {
				handle.await?;
			}
		}


        _ => {
            eprintln!("Unknown command '{}'", command);
        }
    }

    Ok(())
}

/// Sets up a peer connection given a torrent file and a peer address.
async fn setup_peer(file_path: &str, addr: &str) -> Result<(Peer, TcpStream), Box<dyn Error + Send + Sync>> {
    let torrent = Torrent::from_file(file_path)?;
    let peer_id = utils::generate_peer_id();
    let mut peer = Peer::new(torrent.info_hash, peer_id, Some(torrent.info));

    let stream = peer.connect_and_handshake(addr, false).await?;
    if let Some(remote_id) = peer.remote_peer_id {
        println!("Peer ID: {}", hex::encode(remote_id));
    } else {
        println!("Failed to fetch remote id");
    }
    Ok((peer, stream))
}
