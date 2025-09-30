
mod bencode;
mod torrent;
mod tracker;
mod peer;
mod utils;
mod engine;
mod message;
mod piece_manager;
mod piece_queue;
mod magnet;
mod file_io;

use clap::{Parser, Subcommand};
use log::{error, info};
use env_logger;
use std::path::Path;

use crate::engine::{decode_command, info_command, peers_command, handshake_command, download_piece_command, download_command, magnet_parse_command, magnet_handshake_command, magnet_info_command, magnet_download_piece_command, magnet_download_command};

#[derive(Parser)]
#[command(name = "rusbit-cli")]
#[command(about = "A command-line BitTorrent client written in Rust")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Decode a bencoded value
    Decode {
        /// The bencoded string to decode
        bencoded_string: String,
    },
    /// Get information about a torrent file
    Info {
        /// Path to the torrent file
        torrent_file: String,
    },
    /// List peers for a torrent
    Peers {
        /// Path to the torrent file
        torrent_file: String,
    },
    /// Perform handshake with a peer
    Handshake {
        /// Path to the torrent file
        torrent_file: String,
        /// Peer address (ip:port)
        peer_addr: String,
    },
    /// Download a single piece
    DownloadPiece {
        /// Output file path
        #[arg(short, long)]
        output: String,
        /// Path to the torrent file
        torrent_file: String,
        /// Piece index to download
        piece_index: u32,
    },
    /// Download complete torrent
    Download {
        /// Output file path
        #[arg(short, long)]
        output: String,
        /// Path to the torrent file
        torrent_file: String,
    },
    /// Parse a magnet link
    MagnetParse {
        /// The magnet link to parse
        magnet_link: String,
    },
    /// Perform handshake via magnet link
    MagnetHandshake {
        /// The magnet link
        magnet_link: String,
    },
    /// Get info via magnet link
    MagnetInfo {
        /// The magnet link
        magnet_link: String,
    },
    /// Download piece via magnet link
    MagnetDownloadPiece {
        /// Output file path
        #[arg(short, long)]
        output: String,
        /// The magnet link
        magnet_link: String,
        /// Piece index to download
        piece_index: u32,
    },
    /// Download complete torrent via magnet link
    MagnetDownload {
        /// Output file path
        #[arg(short, long)]
        output: String,
        /// The magnet link
        magnet_link: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .init();

    info!("Starting Rusbit CLI v{}", env!("CARGO_PKG_VERSION"));

    let result = match cli.command {
        Commands::Decode { bencoded_string } => {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(decode_command(bencoded_string))
        }
        Commands::Info { torrent_file } => {
            validate_file_path(&torrent_file)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(info_command(torrent_file))
        }
        Commands::Peers { torrent_file } => {
            validate_file_path(&torrent_file)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(peers_command(torrent_file))
        }
        Commands::Handshake { torrent_file, peer_addr } => {
            validate_file_path(&torrent_file)?;
            validate_peer_addr(&peer_addr)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(handshake_command(torrent_file, peer_addr))
        }
        Commands::DownloadPiece { output, torrent_file, piece_index } => {
            validate_file_path(&torrent_file)?;
            validate_output_path(&output)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(download_piece_command(output, torrent_file, piece_index))
        }
        Commands::Download { output, torrent_file } => {
            validate_file_path(&torrent_file)?;
            validate_output_path(&output)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(download_command(output, torrent_file))
        }
        Commands::MagnetParse { magnet_link } => {
            validate_magnet_link(&magnet_link)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(magnet_parse_command(magnet_link))
        }
        Commands::MagnetHandshake { magnet_link } => {
            validate_magnet_link(&magnet_link)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(magnet_handshake_command(magnet_link))
        }
        Commands::MagnetInfo { magnet_link } => {
            validate_magnet_link(&magnet_link)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(magnet_info_command(magnet_link))
        }
        Commands::MagnetDownloadPiece { output, magnet_link, piece_index } => {
            validate_magnet_link(&magnet_link)?;
            validate_output_path(&output)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(magnet_download_piece_command(output, magnet_link, piece_index))
        }
        Commands::MagnetDownload { output, magnet_link } => {
            validate_magnet_link(&magnet_link)?;
            validate_output_path(&output)?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(magnet_download_command(output, magnet_link))
        }
    };

    match result {
        Ok(_) => {
            info!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Command failed: {}", e);
            Err(e)
        }
    }
}

fn validate_file_path(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()).into());
    }
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()).into());
    }
    Ok(())
}

fn validate_output_path(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(format!("Output directory does not exist: {}", parent.display()).into());
        }
    }
    Ok(())
}

fn validate_peer_addr(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    if addr.contains("..") || addr.contains("/") || addr.contains("\\") {
        return Err("Invalid peer address format".into());
    }
    Ok(())
}

fn validate_magnet_link(link: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !link.starts_with("magnet:?") {
        return Err("Invalid magnet link format".into());
    }
    Ok(())
}
