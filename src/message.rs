use anyhow::Result;
use serde::{Serialize, Deserialize, };
use serde_bencode;

#[derive(Serialize, Deserialize)]
pub struct ExtensionHandshakeDict {
    // extension handshake dict
    pub m: MDict, 
}

#[derive(Serialize, Deserialize)]
pub struct MDict {
    pub ut_metadata: u8,
}

pub fn handshake_message(
    info_hash: &[u8],
    peer_id: &[u8],
    enable_extension: bool
) -> Vec<u8> {
    // basic handshake message
    let reserved_bytes: [u8; 8] = match enable_extension {
        true => [0, 0, 0, 0, 0, 16, 0, 0],
        false => [0u8; 8],
    };

    let mut message: Vec<u8> = Vec::new();
    message.push(19); // Protocol string length, 0
    message.extend_from_slice(b"BitTorrent protocol"); // Protocol string, 1 ~ 19
    message.extend_from_slice(&reserved_bytes); // Reserved bytes, 20 ~ 27
    message.extend_from_slice(info_hash); // 28 ~ 47
    message.extend_from_slice(peer_id); // 48 ~ 67
    message
}

pub fn piece_request_message(piece_index: u32, offset: u32, length: u32) -> Vec<u8> {
    let mut message = vec![0u8; 17];
    message[3] = 13; // request message has 13 byres (1 message type, 12 payload)
    message[4] = 6; // message type, 6 -> request type
    message[5..9].copy_from_slice(&piece_index.to_be_bytes());
    message[9..13].copy_from_slice(&offset.to_be_bytes());
    message[13..17].copy_from_slice(&length.to_be_bytes());
    message
}

pub fn extension_handshake_message() -> Result<Vec<u8>> {
    let m_dict = MDict{ ut_metadata: 1 };
    let eh_dict = ExtensionHandshakeDict{ m: m_dict };
    let eh_bytes = serde_bencode::to_bytes(&eh_dict)?;
    let length = eh_bytes.len() as u32 + 2; // including 2 ids
    let length_bytes = length.to_be_bytes();

    let mut message: Vec<u8> = Vec::new();
    message.extend_from_slice(&length_bytes);
    message.push(20); // message id
    message.push(0); // extension id for extension handshake
    message.extend_from_slice(&eh_bytes);
    Ok(message)
}

#[derive(Serialize, Deserialize)]
pub struct ExtensionRequestDict {
    // extension request dict
    msg_type: u32,
    piece: u32,
    total_size: Option<u32>
}

pub fn extension_request_message(metadata_id: u8) -> Result<Vec<u8>> {
    let er_dict = ExtensionRequestDict{ msg_type: 0, piece: 0, total_size: None };
    let er_bytes = serde_bencode::to_bytes(&er_dict)?;
    let length = er_bytes.len() as u32 + 2;
    let length_bytes = length.to_be_bytes();

    let mut message: Vec<u8> = Vec::new();
    message.extend_from_slice(&length_bytes);
    message.push(20); // message id
    message.push(metadata_id); // extension id
    message.extend_from_slice(&er_bytes);
    Ok(message)
}