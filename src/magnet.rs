// magnet.rs
use anyhow::Result;
use reqwest;
use serde::{Serialize, Deserialize};

use crate::encoder;
use crate::peer;

// struct magnet link
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MagnetLink {
    pub xt: String,
    pub dn: String,
    pub tr: String,
}

impl MagnetLink {
    pub fn get_hex_hash(&self) -> String {
        // xt starts with "urn:btih:", 9 characters
        self.xt[9..].to_string()
    }

    pub fn get_hash(&self) -> Result<Vec<u8>> {
        let info_hash = hex::decode(self.xt[9..].to_string())?;
        Ok(info_hash)
    }

    pub fn track_request(&self) -> Result<reqwest::blocking::Response> {
        let url = {
            let tr = &self.tr;
            let length = 999; // a made up value
            let info_hash = {
                let info_hash_bytes = self.get_hash()?;
                encoder::encode_percent(&info_hash_bytes)
            };
            let peer_id = {
                let peer_id_bytes = peer::Peer::gen_peer_id().to_vec();
                encoder::encode_percent(&peer_id_bytes)
            };
            
            format!("{tr}?\
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