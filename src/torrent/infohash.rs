// infohash.rs
use crate::torrent::TorrentInfo;
use crate::bencode::{encode_bvalue, BValue};

use sha1::{Sha1, Digest};

pub fn calculate_info_hash_from_struct(info: &TorrentInfo) -> [u8; 20] {
    // Convert the struct to a BValue::Dict
    let info_bval = info_to_bvalue(info);

    let encoded = encode_bvalue(&info_bval);

    let mut hasher = Sha1::new();
    hasher.update(&encoded);
    let result = hasher.finalize();

    let mut hash_bytes = [0u8; 20];
    hash_bytes.copy_from_slice(&result);
    hash_bytes
}

// turning `TorrentInfo` into a BValue::Dict
fn info_to_bvalue(info: &TorrentInfo) -> BValue {
    use std::collections::HashMap;

    let mut map = HashMap::new();

    // "length"
    map.insert("length".to_string(), BValue::Integer(info.length as i64));

    // "name"
    map.insert("name".to_string(), BValue::ByteString(info.name.clone().into_bytes()));

    // "piece length"
    map.insert("piece length".to_string(), BValue::Integer(info.piece_length as i64));

    // "pieces"
    let mut concat_pieces = Vec::with_capacity(info.pieces.len() * 20);
    for piece_hash in &info.pieces {
        concat_pieces.extend_from_slice(piece_hash);
    }
    map.insert("pieces".to_string(), BValue::ByteString(concat_pieces));

    BValue::Dict(map)
}
