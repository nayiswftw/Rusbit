// lib.rs - Library interface for the BitTorrent CLI

pub mod decoder;
pub mod encoder;
pub mod magnet;
pub mod message;
pub mod peer;
pub mod torrent;
pub mod tracker;
pub mod utils;

// Re-export commonly used types for easier testing
pub use decoder::*;
pub use encoder::*;
pub use magnet::*;
pub use message::*;
pub use peer::*;
pub use torrent::*;
pub use tracker::*;
pub use utils::*;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_decode_simple_bencode() {
        // Test simple integer
        let result = decode_bencoded_value("i42e").unwrap();
        assert_eq!(result, serde_json::json!(42));
        
        // Test simple string
        let result = decode_bencoded_value("4:test").unwrap();
        assert_eq!(result, serde_json::json!("test"));
        
        // Test simple list
        let result = decode_bencoded_value("li1ei2ee").unwrap();
        assert_eq!(result, serde_json::json!([1, 2]));
        
        // Test simple dictionary
        let result = decode_bencoded_value("d3:fooi42ee").unwrap();
        assert_eq!(result, serde_json::json!({"foo": 42}));
    }
    
    #[test]
    fn test_decode_invalid_bencode() {
        // Test incomplete dictionary
        let result = decode_bencoded_value("d");
        assert!(result.is_err());
        
        // Test incomplete string
        let result = decode_bencoded_value("4:ab");
        assert!(result.is_err());
        
        // Test invalid format
        let result = decode_bencoded_value("invalid");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_encode_percent() {
        let input = vec![0x12, 0x34, 0x56];
        let result = encode_percent(&input);
        assert_eq!(result, "%12%34%56");
    }
    
    #[test]
    fn test_magnet_link_parsing() {
        let magnet = "magnet:?xt=urn:btih:1234567890123456789012345678901234567890&dn=test&tr=http://tracker.example.com/announce";
        let result = decode_magnet_link(magnet);
        assert!(result.is_ok());
        let magnet_link = result.unwrap();
        assert_eq!(magnet_link.tr, "http://tracker.example.com/announce");
        assert_eq!(magnet_link.dn, "test");
    }
    
    #[test]
    fn test_udp_tracker_detection() {
        let response = handle_udp_tracker("udp://tracker.example.com:8080/announce").unwrap();
        assert!(response.failure_reason.is_some());
        assert!(response.peers.is_empty());
        assert_eq!(response.interval, 1800);
    }
    
    #[test]
    fn test_tracker_response_creation() {
        use std::net::{Ipv4Addr, SocketAddrV4};
        
        let response = TrackerResponse {
            interval: 900,
            complete: Some(10),
            incomplete: Some(5),
            downloaded: Some(100),
            peers: vec![SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 1), 6881)],
            failure_reason: None,
            warning_message: None,
        };
        
        assert_eq!(response.interval, 900);
        assert_eq!(response.complete, Some(10));
        assert_eq!(response.peers.len(), 1);
    }
}