// torrent.rs

use anyhow::Result;
use reqwest;
use serde::{Serialize, Deserialize};

use crate::encoder;
use crate::peer;

// Decode torrent file
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TorrentFile {
    pub announce: String,
    pub info: TorrentInfo,
}

impl TorrentFile {
    pub fn get_hash(&self) -> Result<Vec<u8>> {
        self.info.get_hash()
    }

    pub fn get_piece_length_real(&self, piece_index: u32) -> u32 {
        self.info.get_piece_length_real(piece_index)
    }

    pub fn get_piece_num(&self) -> usize {
        self.info.get_piece_num()
    }

    pub fn track_request(&self) -> Result<reqwest::blocking::Response> {
        let url = {
            let announce = &self.announce;
            let length = self.info.length;
            let info_hash = {
                let info_hash_bytes = self.get_hash()?;
                encoder::encode_percent(&info_hash_bytes)
            };
            let peer_id = {
                let peer_id_bytes = peer::Peer::gen_peer_id().to_vec();
                encoder::encode_percent(&peer_id_bytes)
            };
            
            format!("{announce}?\
                    info_hash={info_hash}&\
                    peer_id={peer_id}&\
                    port=6881&\
                    uploaded=0&\
                    downloaded=0&\
                    left={length}&\
                    compact=1"
            )
        };

        let raw_response = reqwest::blocking::get(url)?;
        Ok(raw_response)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TorrentInfo {
    pub length: u64,
    pub name: String,
    #[serde(rename="piece length")]
    pub piece_length: u32,
    #[serde(with="serde_bytes")]
    pub pieces: Vec<u8>,
}

impl TorrentInfo {
    pub fn get_hash(&self) -> Result<Vec<u8>> {
        let bencoded_info = encoder::encode_bencode(&self)?;
        let hash = encoder::encode_sha1(&bencoded_info)?;
        Ok(hash)
    }

    pub fn get_piece_length_real(&self, piece_index: u32) -> u32 {
        let piece_length = self.piece_length as u64;
        piece_length.min(self.length - piece_index as u64 * piece_length as u64) as u32
    }

    pub fn get_piece_num(&self) -> usize {
        self.pieces.len() / 20
    }
}