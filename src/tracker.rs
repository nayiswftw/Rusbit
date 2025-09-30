// tracker.rs

use anyhow::Result;
use serde::{Serialize, Deserialize, Deserializer};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4};

// Flexible tracker response structure
#[derive(Serialize, Deserialize, Debug)]
pub struct TrackerResponse {
    #[serde(default = "default_interval")]
    pub interval: i32,
    
    #[serde(default)]
    pub complete: Option<i32>,
    
    #[serde(default)]
    pub incomplete: Option<i32>,
    
    #[serde(default)]
    pub downloaded: Option<i32>,
    
    #[serde(default, deserialize_with = "deserialize_peers_flexible")]
    pub peers: Vec<SocketAddrV4>,
    
    // Handle error responses
    #[serde(default)]
    pub failure_reason: Option<String>,
    
    #[serde(default)]
    pub warning_message: Option<String>,
}

fn default_interval() -> i32 {
    1800 // 30 minutes default
}

// Flexible peer deserialization that handles both compact and dictionary formats
fn deserialize_peers_flexible<'de, D>(deserializer: D) -> Result<Vec<SocketAddrV4>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor, SeqAccess};
    use std::fmt;
    
    struct PeersVisitor;
    
    impl<'de> Visitor<'de> for PeersVisitor {
        type Value = Vec<SocketAddrV4>;
        
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("peers as bytes or list of dictionaries")
        }
        
        // Handle compact format (binary peers)
        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.len() % 6 != 0 {
                return Ok(Vec::new()); // Return empty instead of error for resilience
            }
            
            let mut peers = Vec::with_capacity(v.len() / 6);
            for chunk in v.chunks_exact(6) {
                let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                peers.push(SocketAddrV4::new(ip, port));
            }
            Ok(peers)
        }
        
        // Handle list format (array of peer dictionaries)
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut peers = Vec::new();
            
            while let Some(peer_dict) = seq.next_element::<PeerDict>()? {
                if let (Some(ip_str), Some(port)) = (peer_dict.ip, peer_dict.port) {
                    // Try IPv4 first, then IPv6, then skip if neither works
                    if let Ok(ipv4) = ip_str.parse::<Ipv4Addr>() {
                        peers.push(SocketAddrV4::new(ipv4, port));
                    } else if let Ok(_ipv6) = ip_str.parse::<Ipv6Addr>() {
                        // For IPv6, we need to convert to IPv4 or skip since BitTorrent typically uses IPv4
                        // For now, we'll skip IPv6 addresses but at least won't error
                        continue;
                    }
                }
            }
            
            Ok(peers)
        }
    }
    
    deserializer.deserialize_any(PeersVisitor)
}

#[derive(Deserialize)]
struct PeerDict {
    ip: Option<String>,
    port: Option<u16>,
}

// Note: UDP tracker support could be added here in the future

// Simple UDP tracker handling - returns helpful message
pub fn handle_udp_tracker(tracker_url: &str) -> Result<TrackerResponse> {
    // For now, return a response that explains the situation
    println!("\n🔧 UDP Tracker Detected: {tracker_url}");
    println!("📡 UDP trackers are complex and many are offline.");
    println!("💡 For testing, try creating a torrent with HTTP tracker.");
    println!("🌐 Example: announce URLs starting with 'http://' or 'https://'");
    
    // Return empty response to avoid crashes
    Ok(TrackerResponse {
        interval: 1800,
        complete: Some(0),
        incomplete: Some(0),
        downloaded: None,
        peers: Vec::new(),
        failure_reason: Some("UDP tracker detected - not implemented".to_string()),
        warning_message: Some("Use HTTP/HTTPS tracker for testing".to_string()),
    })
}
