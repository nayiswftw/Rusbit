// main.rs

use anyhow::Result;
use std::{env, str::FromStr};
use tokio;
use tokio::fs::File;

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

// Usage: your_program.sh "command" para1 para2 ...
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value = decoder::decode_bencoded_value(encoded_value)?;
        println!("{}", decoded_value.to_string());
    } else if command == "info" {
        let torrent_file_name = &args[2];
        let torrent = decoder::decode_torrent_file(torrent_file_name)?;
        utils::print_torrent(&torrent)?;
    } else if command == "peers" {
        let torrent_file_name = &args[2];
        let torrent = decoder::decode_torrent_file(torrent_file_name)?;
        let raw_response = torrent.track_request()?.bytes()?;
        let response = decoder::decode_tracker_response(&raw_response)?;
        utils::print_peers(&response);
    } else if command == "handshake" {
        let torrent_file_name = &args[2];
        let peer_addr_str = &args[3];
        let peer_addr = std::net::SocketAddrV4::from_str(peer_addr_str)?;
        let torrent = decoder::decode_torrent_file(torrent_file_name)?;
        let peer = Peer::new(peer_addr, torrent);
        let raw_response = peer.handshake(false).await?;
        println!("Peer ID: {}", hex::encode(&raw_response[48..]));
    } else if command == "download_piece" {
        let file_path = &args[3];
        let torrent_file_name = &args[4];
        let piece_index: u32 = (&args[5]).parse()?;
        let torrent = decoder::decode_torrent_file(torrent_file_name)?; // parse torrent

        let raw_response = torrent.track_request()?.bytes()?;  // get peer info
        let response = decoder::decode_tracker_response(&raw_response)?; // decode tracker response
        let peer_addr = response.peers[0];   // get the first peer

        let file = File::create(file_path).await?;
        file.set_len(torrent.get_piece_length_real(piece_index) as u64).await?;
        peer::download_piece(peer_addr, &torrent.info, piece_index, file_path, 0).await?;    
    } else if command == "download" {
        let file_path = &args[3];
        let torrent_file_name = &args[4];
        let torrent = decoder::decode_torrent_file(torrent_file_name)?;

        let raw_response = torrent.track_request()?.bytes()?;
        let response = decoder::decode_tracker_response(&raw_response)?;
        let piece_num = torrent.get_piece_num();
        let piece_per_peer = piece_num.div_ceil(response.peers.len());

        let mut tasks = Vec::new();
        let file = File::create(file_path).await?;
        file.set_len(torrent.info.length).await?;
        for (peer_index, peer_addr) in response.peers.iter().enumerate() {
            let start_piece = peer_index * piece_per_peer;
            let end_piece = ((peer_index + 1)*piece_per_peer).min(piece_num);
            
            let info_clone = torrent.info.clone();
            let file_path_clone = file_path.to_string();
            let peer_addr_clone = *peer_addr;
            
            let task = tokio::spawn(async move {
                for piece_index  in start_piece..end_piece {
                    let piece_offset = piece_index as u64 * info_clone.piece_length as u64;
                    match peer::download_piece(
                        peer_addr_clone,
                        &info_clone,
                        piece_index as u32,
                        &file_path_clone,
                        piece_offset
                    ).await {
                        Ok(_) => println!("Piece {} downloaded successfully", piece_index),
                        Err(e) => eprintln!("Failed to download piece {}: {}", piece_index, e),
                    }
                }
            });
            tasks.push(task);
        }

        // wait tasks to finish
        for task in tasks {
            task.await?;
        }
        
    } else if command == "magnet_parse" {
        let raw_link = &args[2];
        let magnet_link = decoder::decode_magnet_link(raw_link)?;
        utils::print_magnet(&magnet_link);
    } else if command == "magnet_handshake" {
        let raw_link = &args[2];
        let magnet_link = decoder::decode_magnet_link(raw_link)?;
        let raw_response = magnet_link.track_request()?.bytes()?;  // get peer info
        let response = decoder::decode_tracker_response(&raw_response)?; // decode tracker response
        let peer_addr = response.peers[0];   // get the first peer
        let info_hash = magnet_link.get_hash()?;

        peer::magnet_handshake(peer_addr, &info_hash, true).await?;
    } else if command == "magnet_info" {
        let raw_link = &args[2];
        let magnet_link = decoder::decode_magnet_link(raw_link)?;
        let raw_response = magnet_link.track_request()?.bytes()?;  // get peer info
        let response = decoder::decode_tracker_response(&raw_response)?; // decode tracker response
        let peer_addr = response.peers[0];   // get the first peer
        let info_hash = magnet_link.get_hash()?;

        let info = peer::magnet_request_info(peer_addr, &info_hash, true).await?;
        let torrent = TorrentFile{ announce: magnet_link.tr, info: info };
        utils::print_torrent(&torrent)?;
    } else if command == "magnet_download_piece" {
        let file_path = &args[3];
        let raw_link = &args[4];
        let piece_index: u32 = (&args[5]).parse()?;

        let magnet_link = decoder::decode_magnet_link(raw_link)?;
        let raw_response = magnet_link.track_request()?.bytes()?;  // get peer info
        let response = decoder::decode_tracker_response(&raw_response)?; // decode tracker response
        let peer_addr = response.peers[0];   // get the first peer
        let info_hash = magnet_link.get_hash()?;

        let info = peer::magnet_request_info(peer_addr, &info_hash, true).await?;
        let file = File::create(file_path).await?;
        file.set_len(info.get_piece_length_real(piece_index) as u64).await?;
        peer::download_piece(peer_addr, &info, piece_index, file_path, 0).await?;
    } else if command == "magnet_download" {
        let file_path = &args[3];
        let raw_link = &args[4];
        let magnet_link = decoder::decode_magnet_link(raw_link)?;
        let raw_response = magnet_link.track_request()?.bytes()?; // get peer info
        let response = decoder::decode_tracker_response(&raw_response)?; // decode tracker response
        let peer_addr = response.peers[0]; // get the first peer
        let info_hash = magnet_link.get_hash()?;
        let info = peer::magnet_request_info(peer_addr, &info_hash, true).await?;
        let piece_num = info.get_piece_num();
        let piece_per_peer = piece_num.div_ceil(response.peers.len());
        let mut tasks = Vec::new();
        let file = File::create(file_path).await?;
        file.set_len(info.length).await?;
        for (peer_index, peer_addr) in response.peers.iter().enumerate() {
            let start_piece = peer_index * piece_per_peer;
            let end_piece = ((peer_index + 1) * piece_per_peer).min(piece_num);
            let info_clone = info.clone();
            let file_path_clone = file_path.to_string();
            let peer_addr_clone = *peer_addr;
            let task = tokio::spawn(async move {
                for piece_index in start_piece..end_piece {
                    let piece_offset = piece_index as u64 * info_clone.piece_length as u64;
                    match peer::download_piece(
                        peer_addr_clone,
                        &info_clone,
                        piece_index as u32,
                        &file_path_clone,
                        piece_offset,
                    )
                    .await
                    {
                        Ok(_) => println!("Piece {} downloaded successfully", piece_index),
                        Err(e) => eprintln!("Failed to download piece {}: {}", piece_index, e),
                    }
                }
            });
            tasks.push(task);
        }
        // wait tasks to finish
        for task in tasks {
            task.await?;
        }
    } else {
        println!("unknown command: {}", args[1]);
    }

    Ok(())
}