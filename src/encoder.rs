// encoder.rs

use anyhow::Result;
use serde::Serialize;
use serde_bencode;
use sha1::{Digest, Sha1};

pub fn encode_bencode<T: Serialize>(data: &T) -> Result<Vec<u8>> {
    let encoded_data = serde_bencode::to_bytes(data)?;
    Ok(encoded_data)
}

pub fn encode_sha1(data: &[u8]) -> Result<Vec<u8>> {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let encoded_data = hasher.finalize().to_vec();
    Ok(encoded_data)
}

pub fn encode_percent(data: &[u8]) -> String {
    let mut encoded_data = String::new();
    for &byte in data {
        encoded_data.push_str(&format!("%{:02X}", byte));
    }
    encoded_data
}