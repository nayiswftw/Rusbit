// src/engine.rs
use reqwest::Client;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use log::{info, error};

use crate::bencode::{decode_bencode, bvalue_to_json};
use crate::magnet::decode_magnet;
use crate::torrent::Torrent;
use crate::peer::Peer;
use crate::tracker;
use crate::utils;
use crate::piece_queue::PieceQueue;
use rusbit_cli::progress::ProgressTracker;
pub async fn decode_command(bencoded_string: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    match decode_bencode(bencoded_string.as_bytes()) {
        Ok((_consumed, value)) => {
            let json_val = bvalue_to_json(&value);
            println!("{}", serde_json::to_string(&json_val)?);
            Ok(())
        }
        Err(e) => {
            error!("Error decoding bencoded value: {:?}", e);
            Err(e.into())
        }
    }
}

pub async fn info_command(torrent_file: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    match Torrent::from_file(&torrent_file) {
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
            Ok(())
        }
        Err(err) => {
            error!("Error reading torrent: {:?}", err);
            Err(err.into())
        }
    }
}

pub async fn peers_command(torrent_file: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    let http_client = Client::new();
    let peer_id = utils::generate_peer_id();
    let uploaded = 0u64;
    let downloaded = 0u64;
    let port = 6881;

    let torrent = Torrent::from_file(&torrent_file)?;
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
    Ok(())
}

pub async fn handshake_command(torrent_file: String, peer_addr: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    match setup_peer(&torrent_file, &peer_addr).await {
        Ok((_peer, _stream)) => {
            info!("Handshake successful with peer {}", peer_addr);
            Ok(())
        }
        Err(e) => {
            error!("Error during handshake: {}", e);
            Err(e)
        }
    }
}

pub async fn download_piece_command(output: String, torrent_file: String, piece_index: u32, _show_progress: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
    let http_client = Client::new();
    let peer_id = utils::generate_peer_id();
    let port = 6881;
    let uploaded = 0;
    let downloaded = 0;

    let torrent = Torrent::from_file(&torrent_file)?;
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
        error!("No peers available");
        return Err("No peers available".into());
    }

    let (ip, port) = &potential_peers[0];
    let addr = format!("{}:{}", ip, port);

    // Create a piece queue containing only the one piece we want.
    let piece_queue = Arc::new(PieceQueue::new(VecDeque::from(vec![piece_index])));

    let handle = tokio::spawn(async move {
        while let Some(piece) = piece_queue.get_next_piece().await {
            info!("Peer {} downloading piece {}", addr, piece);

            match setup_peer(&torrent_file, &addr).await {
                Ok((mut peer, stream)) => {
                    if let Err(e) = peer
                        .run_message_loop(stream, piece, &output, Arc::clone(&piece_queue), false, false, None)
                        .await
                    {
                        error!("Error processing messages for {}: {}", addr, e);
                    }
                }
                Err(e) => error!("Failed to setup peer {}: {}", addr, e),
            }
        }
    });
    handle.await?;
    Ok(())
}

pub async fn download_command(output: String, torrent_file: String, show_progress: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
    let http_client = Client::new();
    let peer_id = utils::generate_peer_id();

    let torrent = Torrent::from_file(&torrent_file)?;
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

    // Create progress tracker
    let progress_tracker = Arc::new(ProgressTracker::with_progress_bar(torrent.info.pieces.len(), show_progress));

    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    for (ip, port) in potential_peers {
        let addr = format!("{}:{}", ip, port);
        let torrent_path = torrent_file.clone();
        let output_path = output.clone();
        let pq = Arc::clone(&piece_queue);
        let tracker = Arc::clone(&progress_tracker);

        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            while let Some(piece) = pq.get_next_piece().await {
                info!("Peer {} downloading piece {}", addr, piece);

                match setup_peer(&torrent_path, &addr).await {
                    Ok((mut peer, stream)) => {
                        if let Err(e) = peer
                            .run_message_loop(stream, piece, &output_path, Arc::clone(&pq), true, false, Some(Arc::clone(&tracker)))
                            .await
                        {
                            error!("Error processing messages for {}: {}", addr, e);
                        }
                    }
                    Err(e) => error!("Failed to setup peer {}: {}", addr, e),
                }
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.await?;
    }

    // Finish progress tracking
    progress_tracker.finish();
    Ok(())
}

pub async fn magnet_parse_command(magnet_link: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    let magnet_map = decode_magnet(&magnet_link)?;
    let info_hash = magnet_map.get("info_hash").unwrap();
    let announce = magnet_map.get("announce").unwrap();
    println!("Tracker URL: {}", announce);
    println!("Info Hash: {}", info_hash);
    Ok(())
}

pub async fn magnet_handshake_command(magnet_link: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse magnet link.
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
        false,
        None
    ).await?;

    if let Some(remote_id) = meta_peer.remote_peer_id {
        println!("Peer ID: {}", hex::encode(remote_id));
    } else {
        println!("Failed to fetch remote id");
    }
    Ok(())
}

pub async fn magnet_info_command(magnet_link: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse magnet link.
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

    meta_peer.run_message_loop(
        stream,
        0,
        "test.rs",
        Arc::new(PieceQueue::new(VecDeque::new())),
        false, 
        true,
        None
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
    Ok(())
}

pub async fn magnet_download_piece_command(output: String, magnet_link: String, piece_index: u32, _show_progress: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse magnet link.
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
            None
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
        let output_path_clone = output.clone();

        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            // For each piece, create a new connection.
            while let Some(piece) = piece_queue_clone.get_next_piece().await {
                info!("Peer {} downloading piece {}", addr_clone, piece);

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
                                None
                            )
                            .await
                        {
                            error!("Error processing messages for {}: {}", addr_clone, e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to setup peer {}: {}", addr_clone, e);
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
    Ok(())
}

pub async fn magnet_download_command(output: String, magnet_link: String, show_progress: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse magnet link.
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

    meta_peer.run_message_loop(
        stream,
        0,
        "test.rs",
        Arc::new(PieceQueue::new(VecDeque::new())),
        false, 
        true,
        None
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

    // Create progress tracker
    let progress_tracker = Arc::new(ProgressTracker::with_progress_bar(num_pieces, show_progress));

    // Now spawn download tasks for each available peer.
    let mut handles = Vec::new();
    for (ip, port) in potential_peers {
        let addr = format!("{}:{}", ip, port);
        // Clone the owned data for use in the async task.
        let info_clone = info.clone();
        let piece_queue_clone = Arc::clone(&full_piece_queue);
        let info_hash_bytes_clone = info_hash_bytes;
        let addr_clone = addr.clone();
        let output_path_clone = output.clone();
        let tracker = Arc::clone(&progress_tracker);

        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            // For each piece, create a new connection.
            while let Some(piece) = piece_queue_clone.get_next_piece().await {
                info!("Peer {} downloading piece {}", addr_clone, piece);

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
                                Some(Arc::clone(&tracker))
                            )
                            .await
                        {
                            error!("Error processing messages for {}: {}", addr_clone, e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to setup peer {}: {}", addr_clone, e);
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

    // Finish progress tracking
    progress_tracker.finish();
    Ok(())
}/// Sets up a peer connection given a torrent file and a peer address.
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
