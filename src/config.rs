use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub peer_id_prefix: String,
    pub listen_port: u16,
    pub max_connections: usize,
    pub piece_timeout: u64,
    pub request_timeout: u64,
    pub max_retries: u32,
    pub download_directory: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            peer_id_prefix: "-RB0001-".to_string(),
            listen_port: 6881,
            max_connections: 50,
            piece_timeout: 30, // seconds
            request_timeout: 10, // seconds
            max_retries: 3,
            download_directory: ".".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = "rusbit.toml";
        if Path::new(config_path).exists() {
            let contents = fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            let config = Self::default();
            // Save default config
            let toml = toml::to_string(&config)?;
            fs::write(config_path, toml)?;
            Ok(config)
        }
    }
}