// peer.rs

use anyhow::Result;
use rand;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::fs::File;
use tokio::net::TcpStream;

use std::net::SocketAddrV4;

use crate::message::{self, ExtensionHandshakeDict, ExtensionRequestDict};
use crate::torrent::{TorrentFile, TorrentInfo};

// peer struct
pub struct Peer {
    pub peer_addr: SocketAddrV4,
    pub torrent: TorrentFile,
}

impl Peer {
    pub fn new(peer_addr: SocketAddrV4, torrent: TorrentFile) -> Self {
        Self{ peer_addr, torrent}
    }

    // Handshake with peer
    pub async fn handshake(&self, enable_extension: bool) -> Result<Vec<u8>> {
        // Setup tcp connection
        let mut stream = TcpStream::connect(self.peer_addr).await?;

        // Construct handshake message
        let handshake_message = {
            let reserved_bytes: [u8; 8] = match enable_extension {
                true => [0, 0, 0, 0, 0, 16, 0, 0],
                false => [0u8; 8],
            };

            let mut message: Vec<u8> = Vec::new();
            message.push(19); // Protocol string length
            message.extend_from_slice(b"BitTorrent protocol"); // Protocol string
            message.extend_from_slice(&reserved_bytes); // Reserved bytes
            message.extend_from_slice(&self.torrent.get_hash()?);
            message.extend_from_slice(&Self::gen_peer_id());
            message
        };
        
        // Send handshake message and read response
        stream.write_all(&handshake_message).await?;
        let mut buffer = [0u8; 68];
        stream.read(&mut buffer).await?;
        Ok(buffer.to_vec())
    }

    // randomly generate a peer id
    pub fn gen_peer_id() -> [u8; 20] {
        let mut peer_id = [0u8; 20];
        for idx in 0..peer_id.len() {
            peer_id[idx] = rand::random_range(0..255);
        }
        peer_id
    }
}



// Download a piece from peer
pub async fn download_piece(
    peer_addr: SocketAddrV4,
    info: &TorrentInfo,
    piece_index: u32,
    file_path: &str,
    piece_offset: u64
) -> Result<()> {
    // Step1 connect peer
    let mut stream = TcpStream::connect(peer_addr).await?;
    let peer_id = Peer::gen_peer_id();
    let info_hash = info.get_hash()?;

    // Step2 send handshake message
    let handshake_message = message::handshake_message(&info_hash, &peer_id, false);
    stream.write_all(&handshake_message).await?;

    // Step3 read handshake response
    let mut buffer = [0u8; 68];
    stream.read_exact(&mut buffer).await?;
    
    // Step4 read bitfield message
    let mut length_bytes = [0u8; 4];
    stream.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes);
    let mut buffer = vec![0u8; length as usize];
    stream.read_exact(&mut buffer).await?;

    // Step5 send interest message
    let message: [u8; 5] = [0, 0, 0, 1, 2];
    stream.write_all(&message).await?;

    // Step6 recive unchocked message
    let mut length_bytes = [0u8; 4];
    stream.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes);
    let mut buffer = vec![0u8; length as usize];
    stream.read_exact(&mut buffer).await?;

    // Step7 send piece requests
    let piece_length = info.get_piece_length_real(piece_index);
    let block_size: u32 = 16384;
    let blocks = piece_length.div_ceil(block_size);

    for block_idx in 0..blocks {
        let offset = block_idx * block_size;
        let length = 16384u32.min(piece_length - offset);
        let request_message = message::piece_request_message(piece_index, offset, length);
        stream.write_all(&request_message).await?;
    }

    // Step8 read piece data
    let mut piece_buffer = vec![0u8; piece_length as usize];
    for _ in 0..blocks {
        let mut length_bytes = [0u8; 4];
        stream.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes);
        let mut buffer = vec![0u8; length as usize];
        stream.read_exact(&mut buffer).await?;

        let offset = u32::from_be_bytes([buffer[5], buffer[6], buffer[7], buffer[8]]) as usize;
        let block_data = &buffer[9..];
        let (start, end) = (offset, offset + block_data.len());
        piece_buffer[start..end].copy_from_slice(block_data);
    }

    // Step9 save piece file
    let mut file = File::options().write(true).open(file_path).await?;
    file.seek(SeekFrom::Start(piece_offset)).await?;
    file.write_all(&piece_buffer).await?;
    file.flush().await?;

    Ok(())
}

// magnet handshake (support extension) 
pub async fn magnet_handshake(
    peer_addr: SocketAddrV4,
    info_hash: &[u8],
    enable_extension: bool,
) -> Result<()> {
    let mut stream = TcpStream::connect(peer_addr).await?;
    let peer_id = Peer::gen_peer_id();

    // Create handshake message and send
    let handshake_message = message::handshake_message(info_hash, &peer_id, enable_extension);
    stream.write_all(&handshake_message).await?;
    
    // Read handshake response
    let mut buffer = [0u8; 68];
    stream.read(&mut buffer).await?;
    println!("Peer ID: {}", hex::encode(&buffer[48..]));
    
    // if support extension
    if buffer[25] == 16 {
        // Read bitfield message
        let mut length_bytes = [0u8; 4];
        stream.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes);
        let mut buffer = vec![0u8; length as usize];
        stream.read_exact(&mut buffer).await?;

        // Create extension message and send
        let extension_message = message::extension_handshake_message()?;
        stream.write_all(&extension_message).await?;

        // Read extension response
        let mut length_bytes = [0u8; 4];
        stream.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes);
        let mut buffer = vec![0u8; length as usize];
        stream.read_exact(&mut buffer).await?;

        let eh_dict: ExtensionHandshakeDict = serde_bencode::from_bytes(&buffer[2..])?;
        println!("Peer Metadata Extension ID: {}", eh_dict.m.ut_metadata);
    }

    Ok(())
}

// magnet handshake (support extension) 
pub async fn magnet_request_info(
    peer_addr: SocketAddrV4,
    info_hash: &[u8],
    enable_extension: bool,
) -> Result<TorrentInfo> {
    let mut stream = TcpStream::connect(peer_addr).await?;
    let peer_id = Peer::gen_peer_id();

    // Create handshake message and send
    let handshake_message = message::handshake_message(info_hash, &peer_id, enable_extension);
    stream.write_all(&handshake_message).await?;
    
    // Read handshake response
    let mut buffer = [0u8; 68];
    stream.read(&mut buffer).await?;
    
    // Read bitfield message
    let mut length_bytes = [0u8; 4];
    stream.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes);
    let mut buffer = vec![0u8; length as usize];
    stream.read_exact(&mut buffer).await?;

    // Create extension handshake message and send
    let eh_message = message::extension_handshake_message()?;
    stream.write_all(&eh_message).await?;

    // Read extension handshake response
    let mut length_bytes = [0u8; 4];
    stream.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes);
    let mut buffer = vec![0u8; length as usize];
    stream.read_exact(&mut buffer).await?;
    let ehr_dict: ExtensionHandshakeDict = serde_bencode::from_bytes(&buffer[2..])?;
    let metadata_id = ehr_dict.m.ut_metadata;

    // Create extension request message and send
    let er_message = message::extension_request_message(metadata_id)?;
    stream.write_all(&er_message).await?;

    // Read extension request response
    let mut length_bytes = [0u8; 4];
    stream.read_exact(&mut length_bytes).await?;
    let length = u32::from_be_bytes(length_bytes);
    let mut buffer = vec![0u8; length as usize];
    stream.read_exact(&mut buffer).await?;

    type ERR = (ExtensionRequestDict, TorrentInfo);
    let mut completed_bytes = vec![b'l'];
    completed_bytes.extend_from_slice(&buffer[2..]);
    completed_bytes.push(b'e');
    let err_tuple: ERR = serde_bencode::from_bytes(&completed_bytes)?;
    Ok(err_tuple.1)
}