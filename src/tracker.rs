// tracker.rs

use anyhow::Result;
use serde::{Serialize, Deserialize, Deserializer};
use serde_bytes;

use std::net::{Ipv4Addr, SocketAddrV4};

// Tracker response struct
#[derive(Serialize, Deserialize, Debug)]
pub struct TrackerResponse {
    pub interval: i32,
    #[serde(deserialize_with = "deserialize_peers")]
    pub peers: Vec<SocketAddrV4>,
}

fn deserialize_peers<'de, D>(deserializer: D) -> Result<Vec<SocketAddrV4>, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes: Vec<u8> = serde_bytes::deserialize(deserializer)?;
    let mut peers: Vec<SocketAddrV4> = Vec::new();
    
    for chunk in bytes.chunks(6) {
        let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        peers.push(SocketAddrV4::new(ip, port));
    }
    
    Ok(peers)
}
