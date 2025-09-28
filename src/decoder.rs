// decoder.rs

use bytes::Bytes;
use serde_json;
use serde_bencode;
use serde_urlencoded;
use anyhow::Result;
use std::{fs, collections};

use crate::magnet::MagnetLink;
use crate::torrent::TorrentFile;
use crate::tracker::TrackerResponse;

// Decode bencoded data
pub fn decode_bencoded_value(encoded_value: &str) -> Result<serde_json::Value> {
    let value: serde_bencode::value::Value = serde_bencode::from_str(encoded_value)?;
    convert(value)
}

fn convert(value: serde_bencode::value::Value) -> Result<serde_json::Value> {
    match value {
        serde_bencode::value::Value::Int(i) => {
            Ok(serde_json::to_value(i)?)
        },
        serde_bencode::value::Value::Bytes(b) => {
            let string = String::from_utf8(b)?;
            Ok(serde_json::to_value(string)?)
        },
        serde_bencode::value::Value::List(l) => {
            let array = l.into_iter()
                .map(|elem| convert(elem))
                .collect::<Result<Vec<serde_json::Value>>>()?;
            Ok(serde_json::to_value(array)?)
        },
        serde_bencode::value::Value::Dict(d) => {
            let dict = d.into_iter()
                .map(|(_key, _val)| {
                    let key = String::from_utf8(_key)?;
                    let val = convert(_val)?;
                    Ok((key, val))
                })
                .collect::<Result<collections::HashMap<String, serde_json::Value>>>()?;
            
            Ok(serde_json::to_value(dict)?)
        }
    }
}

// Decode torrent file
pub fn decode_torrent_file(file_name: &str) -> Result<TorrentFile> {
    let content_encoded = fs::read(file_name)?;
    let content: TorrentFile = serde_bencode::from_bytes(&content_encoded)?;
    Ok(content)
}

// Decode tracker response
pub fn decode_tracker_response(raw_response: &Bytes) -> Result<TrackerResponse> {
    let content: TrackerResponse = serde_bencode::from_bytes(raw_response)?;
    Ok(content)
}

// Decode magnet link
pub fn decode_magnet_link(raw_link: &str) -> Result<MagnetLink> {
    let clean_link = match raw_link.starts_with("magnet:?") {
        true => raw_link.strip_prefix("magnet:?").unwrap(),
        false => raw_link,
    };
    let content: MagnetLink = serde_urlencoded::from_str(clean_link)?;
    Ok(content)
}