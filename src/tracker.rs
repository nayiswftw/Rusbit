use reqwest::Client;
use crate::bencode::{BValue, decode_bencode};
use crate::utils::url_encode_bytes;
use std::error::Error;

/// Announces to the tracker's `announce` URL and returns a list of peers (IP+port).
///
/// * `announce`: The tracker URL.
/// * `info_hash`: The info hash bytes.
/// * `peer_id`: The 20-byte peer ID you’re using.
/// * `uploaded`: Bytes uploaded so far.
/// * `downloaded`: Bytes downloaded so far.
/// * `left`: Bytes left to download.
/// * `port`: Port number.
///
/// Returns a vector of (ip, port) pairs or an error.
pub async fn announce(
    client: &Client,
    announce: &str,
    info_hash: &[u8],
    peer_id: &[u8; 20],
    uploaded: u64,
    downloaded: u64,
    left: u64,
    port: u16,
) -> Result<Vec<(String, u16)>, Box<dyn Error + Send + Sync>> {
    let info_hash_encoded = url_encode_bytes(info_hash);
    let peer_id_encoded = url_encode_bytes(peer_id);

    let url = format!(
        "{announce}?info_hash={info_hash}&peer_id={peer_id}&port={port}&uploaded={uploaded}&downloaded={downloaded}&left={left}&compact=1",
        announce   = announce,
        info_hash  = info_hash_encoded,
        peer_id    = peer_id_encoded,
        port       = port,
        uploaded   = uploaded,
        downloaded = downloaded,
        left       = left
    );

    let response_bytes = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Tracker request failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Reading tracker response failed: {e}"))?
        .to_vec();

    let (_len, bvalue) = decode_bencode(&response_bytes)
        .map_err(|e| format!("Tracker response bencode error: {e:?}"))?;

    // Check if the tracker returned a failure reason.
    if let BValue::Dict(ref dict) = bvalue {
        if let Some(BValue::ByteString(reason)) = dict.get("failure reason") {
            let failure_str = String::from_utf8_lossy(reason);
            return Err(format!("Tracker failure: {failure_str}").into());
        }
    }

    let peer_list = parse_peers_from_bvalue(&bvalue)?;
    Ok(peer_list)
}

/// Parses a `BValue` (which should be the top-level dictionary from the tracker response)
/// to extract either a "compact" or "non-compact" list of peers.
fn parse_peers_from_bvalue(bval: &BValue) -> Result<Vec<(String, u16)>, Box<dyn Error + Send + Sync>> {
    let dict = match bval {
        BValue::Dict(d) => d,
        _ => return Err("Tracker response not a dictionary".into()),
    };

    // The "peers" key can be a ByteString (compact) or a List of Dicts (non-compact).
    let peers_val = dict.get("peers")
        .ok_or_else(|| "Missing 'peers' key in tracker response".to_string())?;

    match peers_val {
        // Compact mode: each peer is 6 bytes: [IP(4), Port(2)]
        BValue::ByteString(bytes) => {
            if bytes.len() % 6 != 0 {
                return Err("Invalid compact peers length".into());
            }

            let mut result = Vec::new();
            for chunk in bytes.chunks_exact(6) {
                let ip = format!("{}.{}.{}.{}", chunk[0], chunk[1], chunk[2], chunk[3]);
                let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                result.push((ip, port));
            }
            Ok(result)
        }
        // Non-compact: a List of dicts, each with "ip" and "port"
        BValue::List(list) => {
            let mut result = Vec::new();
            for item in list {
                if let BValue::Dict(peer_dict) = item {
                    let ip = match peer_dict.get("ip") {
                        Some(BValue::ByteString(ip_bytes)) => {
                            String::from_utf8_lossy(ip_bytes).to_string()
                        }
                        _ => continue,
                    };
                    let port = match peer_dict.get("port") {
                        Some(BValue::Integer(num)) => *num as u16,
                        _ => continue,
                    };
                    result.push((ip, port));
                }
            }
            Ok(result)
        }
        _ => Err("'peers' is neither ByteString nor List".into()),
    }
}
