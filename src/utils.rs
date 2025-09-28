// utils.rs

use anyhow::Result;
use hex;

use crate::magnet::MagnetLink;
use crate::torrent::TorrentFile;
use crate::tracker::TrackerResponse;

// Command "info" printing
pub fn print_torrent(torrent: &TorrentFile) -> Result<()> {
    // print tracker url and info length
    println!("Tracker URL: {}", torrent.announce);
    println!("Length: {}", torrent.info.length);

    // print info hash
    let info_hash = torrent.get_hash()?;
    let info_hash_string = hex::encode(info_hash);
    println!("Info Hash: {info_hash_string}");

    // print piece length and each piece content (20 bytes hash)
    println!("Piece Length: {}", torrent.info.piece_length);
    println!("Piece Hashes:");
    let chunk_size = 20usize;
    for piece_hash in torrent.info.pieces.chunks(chunk_size) {
        println!("{}", hex::encode(piece_hash));
    }

    Ok(())
}

// Command "peers" printing
pub fn print_peers(tracker_response: &TrackerResponse) {
    for socket_addr in tracker_response.peers.iter() {
        println!("{socket_addr}");
    }
}

// Command "magnet_parse" printing
pub fn print_magnet(magnet_info: &MagnetLink) {
    println!("Tracker URL: {}", magnet_info.tr);
    println!("Info Hash: {}", magnet_info.get_hex_hash());
}