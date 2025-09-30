// main.rs

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::fs::File;
use std::str::FromStr;

mod decoder;
mod encoder;
mod peer;
mod magnet;
mod message;
mod torrent;
mod tracker;
mod utils;

use peer::Peer;
use torrent::TorrentFile;

#[derive(Parser)]
#[command(name = "rusbit-cli")]
#[command(about = "A BitTorrent client CLI tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Decode a bencoded value
    Decode {
        /// The bencoded value to decode
        value: String,
    },
    /// Show information about a torrent file
    Info {
        /// Path to the torrent file
        torrent_file: String,
    },
    /// Get peers for a torrent file
    Peers {
        /// Path to the torrent file
        torrent_file: String,
    },
    /// Perform handshake with a peer
    Handshake {
        /// Path to the torrent file
        torrent_file: String,
        /// Peer address (IP:port)
        peer_addr: String,
    },
    /// Download a specific piece
    DownloadPiece {
        /// Output file path
        output_file: String,
        /// Path to the torrent file
        torrent_file: String,
        /// Piece index to download
        piece_index: u32,
    },
    /// Download the entire file
    Download {
        /// Output file path
        output_file: String,
        /// Path to the torrent file
        torrent_file: String,
    },
    /// Parse a magnet link
    MagnetParse {
        /// The magnet link URL
        link: String,
    },
    /// Perform handshake with a peer using magnet link
    MagnetHandshake {
        /// The magnet link URL
        link: String,
    },
    /// Get torrent info using magnet link
    MagnetInfo {
        /// The magnet link URL
        link: String,
    },
    /// Download a specific piece using magnet link
    MagnetDownloadPiece {
        /// Output file path
        output_file: String,
        /// The magnet link URL
        link: String,
        /// Piece index to download
        piece_index: u32,
    },
    /// Download the entire file using magnet link
    MagnetDownload {
        /// Output file path
        output_file: String,
        /// The magnet link URL
        link: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { value } => {
            let decoded_value = decoder::decode_bencoded_value(&value)?;
            println!("{decoded_value}");
        }
        Commands::Info { torrent_file } => {
            let torrent = decoder::decode_torrent_file(&torrent_file)?;
            utils::print_torrent(&torrent)?;
        }
        Commands::Peers { torrent_file } => {
            let torrent = decoder::decode_torrent_file(&torrent_file)?;
            
            if torrent.announce.starts_with("udp://") {
                // Handle UDP tracker with simple message
                let response = torrent.handle_udp_tracker()?;
                utils::print_peers(&response);
            } else {
                // Handle HTTP/HTTPS tracker
                let (status, response_bytes) = tokio::task::spawn_blocking(move || -> Result<(reqwest::StatusCode, bytes::Bytes)> {
                    let response = torrent.track_request()?;
                    let status = response.status();
                    let bytes = response.bytes()?;
                    Ok((status, bytes))
                }).await??;
                
                // Check if we got an HTTP error status
                if !status.is_success() {
                    println!("Tracker returned HTTP error: {} {}", 
                        status.as_u16(), 
                        status.canonical_reason().unwrap_or("Unknown"));
                    
                    let response_text = String::from_utf8_lossy(&response_bytes);
                    
                    if response_text.contains("503") || response_text.contains("Service Unavailable") {
                        println!("The tracker is temporarily unavailable. This is normal for public trackers that may be overloaded.");
                        println!("Try again later or use a different torrent file with working trackers.");
                    } else {
                        println!("Tracker response: {}", response_text.chars().take(200).collect::<String>());
                    }
                    return Ok(());
                }
                
                match decoder::decode_tracker_response(&response_bytes) {
                    Ok(response) => utils::print_peers(&response),
                    Err(e) => {
                        println!("Failed to parse tracker response: {e}");
                        println!("This could be due to an unsupported tracker response format or network issues.");
                    }
                }
            }
        }
        Commands::Handshake { torrent_file, peer_addr } => {
            let peer_addr = std::net::SocketAddrV4::from_str(&peer_addr)?;
            let torrent = decoder::decode_torrent_file(&torrent_file)?;
            let peer = Peer::new(peer_addr, torrent);
            let raw_response = peer.handshake(false).await?;
            println!("Peer ID: {}", hex::encode(&raw_response[48..]));
        }
        Commands::DownloadPiece { output_file, torrent_file, piece_index } => {
            let torrent = decoder::decode_torrent_file(&torrent_file)?;

            if torrent.announce.starts_with("udp://") {
                // Handle UDP tracker with informative message
                println!("\n🔧 UDP Tracker Detected: {}", torrent.announce);
                println!("📡 UDP trackers are complex and many are offline.");
                println!("💡 For downloading, try creating a torrent with HTTP tracker.");
                println!("🌐 Example: announce URLs starting with 'http://' or 'https://'");
                println!("\nAlternatively, you can:");
                println!("  1. Use magnet links with HTTP trackers");
                println!("  2. Find torrents with working HTTP trackers");
                println!("  3. Use the 'peers' command to check tracker status");
                anyhow::bail!("Cannot download with UDP tracker - use HTTP tracker instead");
            }

            let torrent_clone = torrent.clone();
            let raw_response = tokio::task::spawn_blocking(move || -> Result<bytes::Bytes> {
                Ok(torrent_clone.track_request()?.bytes()?)
            }).await??;
            let response = decoder::decode_tracker_response(&raw_response)?;
            if response.peers.is_empty() {
                anyhow::bail!("No peers available");
            }
            let peer_addr = response.peers[0];
            let file = File::create(&output_file).await?;
            file.set_len(torrent.get_piece_length_real(piece_index) as u64).await?;
            peer::download_piece(peer_addr, &torrent.info, piece_index, &output_file, 0).await?;
        }
        Commands::Download { output_file, torrent_file } => {
            let torrent = decoder::decode_torrent_file(&torrent_file)?;

            if torrent.announce.starts_with("udp://") {
                // Handle UDP tracker with informative message
                println!("\n🔧 UDP Tracker Detected: {}", torrent.announce);
                println!("📡 UDP trackers are complex and many are offline.");
                println!("💡 For downloading, try creating a torrent with HTTP tracker.");
                println!("🌐 Example: announce URLs starting with 'http://' or 'https://'");
                println!("\nAlternatively, you can:");
                println!("  1. Use magnet links with HTTP trackers");
                println!("  2. Find torrents with working HTTP trackers");
                println!("  3. Use the 'peers' command to check tracker status");
                anyhow::bail!("Cannot download with UDP tracker - use HTTP tracker instead");
            }

            let torrent_clone = torrent.clone();
            let raw_response = tokio::task::spawn_blocking(move || -> Result<bytes::Bytes> {
                Ok(torrent_clone.track_request()?.bytes()?)
            }).await??;
            let response = decoder::decode_tracker_response(&raw_response)?;
            if response.peers.is_empty() {
                anyhow::bail!("No peers available");
            }
            download_file(&output_file, &torrent, &response.peers).await?;
        }
        Commands::MagnetParse { link } => {
            let magnet_link = decoder::decode_magnet_link(&link)?;
            utils::print_magnet(&magnet_link);
        }
        Commands::MagnetHandshake { link } => {
            let magnet_link = decoder::decode_magnet_link(&link)?;
            let magnet_clone = magnet_link.clone();
            let raw_response = tokio::task::spawn_blocking(move || -> Result<bytes::Bytes> {
                Ok(magnet_clone.track_request()?.bytes()?)
            }).await??;
            let response = decoder::decode_tracker_response(&raw_response)?;
            if response.peers.is_empty() {
                anyhow::bail!("No peers available");
            }
            let peer_addr = response.peers[0];
            let info_hash = magnet_link.get_hash()?;
            peer::magnet_handshake(peer_addr, &info_hash, true).await?;
        }
        Commands::MagnetInfo { link } => {
            let magnet_link = decoder::decode_magnet_link(&link)?;
            let magnet_clone = magnet_link.clone();
            let raw_response = tokio::task::spawn_blocking(move || -> Result<bytes::Bytes> {
                Ok(magnet_clone.track_request()?.bytes()?)
            }).await??;
            let response = decoder::decode_tracker_response(&raw_response)?;
            if response.peers.is_empty() {
                anyhow::bail!("No peers available");
            }
            let peer_addr = response.peers[0];
            let info_hash = magnet_link.get_hash()?;
            let info = peer::magnet_request_info(peer_addr, &info_hash, true).await?;
            let torrent = TorrentFile::new(magnet_link.tr, info);
            utils::print_torrent(&torrent)?;
        }
        Commands::MagnetDownloadPiece { output_file, link, piece_index } => {
            let magnet_link = decoder::decode_magnet_link(&link)?;
            let magnet_clone = magnet_link.clone();
            let raw_response = tokio::task::spawn_blocking(move || -> Result<bytes::Bytes> {
                Ok(magnet_clone.track_request()?.bytes()?)
            }).await??;
            let response = decoder::decode_tracker_response(&raw_response)?;
            if response.peers.is_empty() {
                anyhow::bail!("No peers available");
            }
            let peer_addr = response.peers[0];
            let info_hash = magnet_link.get_hash()?;
            let info = peer::magnet_request_info(peer_addr, &info_hash, true).await?;
            let file = File::create(&output_file).await?;
            file.set_len(info.get_piece_length_real(piece_index) as u64).await?;
            peer::download_piece(peer_addr, &info, piece_index, &output_file, 0).await?;
        }
        Commands::MagnetDownload { output_file, link } => {
            let magnet_link = decoder::decode_magnet_link(&link)?;
            let magnet_clone = magnet_link.clone();
            let raw_response = tokio::task::spawn_blocking(move || -> Result<bytes::Bytes> {
                Ok(magnet_clone.track_request()?.bytes()?)
            }).await??;
            let response = decoder::decode_tracker_response(&raw_response)?;
            if response.peers.is_empty() {
                anyhow::bail!("No peers available");
            }
            let peer_addr = response.peers[0];
            let info_hash = magnet_link.get_hash()?;
            let info = peer::magnet_request_info(peer_addr, &info_hash, true).await?;
            let torrent = TorrentFile::new(magnet_link.tr, info);
            download_file(&output_file, &torrent, &response.peers).await?;
        }
    }

    Ok(())
}

async fn download_file(output_file: &str, torrent: &TorrentFile, peers: &[std::net::SocketAddrV4]) -> Result<()> {
    let piece_num = torrent.get_piece_num();
    println!("Starting download of {} pieces using {} peers", piece_num, peers.len());

    // Limit concurrent connections to avoid overwhelming peers
    let max_concurrent_peers = peers.len().min(8);
    let active_peers = &peers[..max_concurrent_peers];
    
    let pieces_per_peer = piece_num.div_ceil(active_peers.len());
    let mut tasks = Vec::with_capacity(active_peers.len());
    
    // Pre-allocate file with correct size
    let file = tokio::fs::File::create(output_file).await?;
    file.set_len(torrent.info.get_total_length()).await?;
    drop(file); // Close handle before spawning tasks

    for (peer_index, &peer_addr) in active_peers.iter().enumerate() {
        let start_piece = peer_index * pieces_per_peer;
        let end_piece = ((peer_index + 1) * pieces_per_peer).min(piece_num);
        let info = torrent.info.clone(); // Only clone what's necessary
        let file_path = output_file.to_string();

        let task = tokio::spawn(async move {
            let mut success_count = 0;
            let mut consecutive_failures = 0;
            const MAX_CONSECUTIVE_FAILURES: usize = 3;
            
            for piece_index in start_piece..end_piece {
                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    eprintln!("Peer {peer_addr} has too many consecutive failures, skipping remaining pieces");
                    break;
                }
                
                let piece_offset = piece_index as u64 * info.piece_length as u64;
                match peer::download_piece(
                    peer_addr,
                    &info,
                    piece_index as u32,
                    &file_path,
                    piece_offset
                ).await {
                    Ok(_) => {
                        success_count += 1;
                        consecutive_failures = 0;
                        println!("Piece {piece_index} downloaded successfully (peer: {peer_addr})");
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        eprintln!("Failed to download piece {piece_index} from {peer_addr}: {e}");
                    }
                }
            }
            (peer_index, success_count, end_piece - start_piece)
        });
        tasks.push(task);
    }

    let mut total_success = 0;
    let mut total_expected = 0;
    for task in tasks {
        let (peer_index, success_count, expected_count) = task.await?;
        total_success += success_count;
        total_expected += expected_count;
        println!("Peer {peer_index} completed: {success_count}/{expected_count} pieces");
    }

    println!("Download completed: {total_success}/{total_expected} pieces downloaded successfully");
    if total_success < total_expected {
        println!("Warning: Some pieces failed to download. The file may be incomplete.");
    }

    Ok(())
}