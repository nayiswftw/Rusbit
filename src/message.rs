
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::{
		collections::HashMap, 
		io::{Error, ErrorKind}
};
use crate::bencode::{decode_bencode, BValue, encode_bvalue};

/// Constants for the BitTorrent protocol handshake.
pub const BT_PROTOCOL_STR: &str = "BitTorrent protocol";
pub const BT_PROTOCOL_LEN: u8 = 19;

/// The wire message types we support.
#[derive(Debug)]
pub enum Message {
    /// Standard messages
    Interested,
    Unchoke,
    Bitfield,
    Request { index: u32, begin: u32, length: u32 },
    Piece { payload: Vec<u8> },
    /// Extended messages
    ExtendedHandshake(BValue),
	ReceiveMetaData { ext_msg_id: u8, dict: BValue, payload: Vec<u8> },
	RequestMetaData { ext_msg_id: u8, payload: Vec<u8> },
}

/// Sends a non-handshake message.
pub async fn send_message<S>(stream: &mut S, message: Message) -> Result<(), Error>
where
    S: AsyncWrite + Unpin,
{
    match message {
        Message::Interested => {
            // Interested message: length = 1 (id) + 0 payload
            let msg = [0, 0, 0, 1, 2]; // “2” is the Interested message id
            stream.write_all(&msg).await?;
        }
        Message::Request { index, begin, length } => {
            // Request message: length (4 bytes) + id (1 byte = 6) + 12 bytes payload
            let mut msg = Vec::with_capacity(17);
            msg.extend_from_slice(&13_u32.to_be_bytes());
            msg.push(6); // Request message id is 6.
            msg.extend_from_slice(&index.to_be_bytes());
            msg.extend_from_slice(&begin.to_be_bytes());
            msg.extend_from_slice(&length.to_be_bytes());
            stream.write_all(&msg).await?;
        }
		Message::RequestMetaData { ext_msg_id, payload }  => {
            let mut msg: Vec<u8> = Vec::new();

			let length = ((payload.len() + 2) as u32).to_be_bytes();
			msg.extend_from_slice(&length);
			msg.push(20);
			msg.extend_from_slice(&[ext_msg_id]);
			msg.extend_from_slice(&payload);
            stream.write_all(&msg).await?;
		}
        _ => unimplemented!("send_message not implemented for {:?}", message),
    }
    stream.flush().await?;
    Ok(())
}

/// Reads a message from the stream and converts it into our `Message` enum.
pub async fn read_message<S>(stream: &mut S) -> Result<Message, Error>
where
    S: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let length = u32::from_be_bytes(len_buf) as usize;
    let mut msg_buf = vec![0u8; length];
    stream.read_exact(&mut msg_buf).await?;
    
    // The first byte is the message id.
    let msg_id = msg_buf[0];
    let payload: Vec<u8> = msg_buf[1..].to_vec();
    match msg_id {
        1 => Ok(Message::Unchoke),
        2 => Ok(Message::Interested),
        5 => Ok(Message::Bitfield),
        7 => Ok(Message::Piece { payload }),
        20 => {
            // For extended messages, the payload must start with an extension message id.
            if payload.is_empty() {
                return Err(Error::new(ErrorKind::InvalidData, "Empty extended message payload"));
            }
            // The first byte is the extension message id.
            let ext_msg_id = payload[0];
            let ext_payload = &payload[1..];
            if ext_msg_id == 0 {
                // This is the extended handshake.
                let (_consumed, bvalue) = decode_bencode(ext_payload)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
                Ok(Message::ExtendedHandshake(bvalue))
			} else {
                // This is a metadata data message.
                // Decode the bencoded dictionary at the start of ext_payload.
                let (consumed, dict) = decode_bencode(ext_payload)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

				let payload =  ext_payload[consumed..].to_vec();
				Ok(Message::ReceiveMetaData { ext_msg_id, dict, payload})
			}
        }
        _ => Err(Error::new(ErrorKind::Other, "Unknown message id")),
    }
}

/// Sends the BitTorrent handshake.
pub async fn send_handshake<S>(
    stream: &mut S,
    info_hash: &[u8; 20],
    peer_id: &[u8; 20],
    extension: bool,
) -> Result<(), Error>
where
    S: AsyncWrite + Unpin,
{
    let mut handshake = Vec::with_capacity(68);
    handshake.push(BT_PROTOCOL_LEN);
    handshake.extend_from_slice(BT_PROTOCOL_STR.as_bytes());

    let mut reserved:[u8; 8] = [0u8; 8];
    if extension {
        reserved[5] = 0x10; // Enable extension protocol.
    }
    handshake.extend_from_slice(&reserved);
    handshake.extend_from_slice(info_hash);
    handshake.extend_from_slice(peer_id);

    stream.write_all(&handshake).await?;
    stream.flush().await?;
    Ok(())
}

// Send an extended handshake message.
/// This message has a header byte (0x14 for extended messages) and an extended id of 0.
/// The payload is a bencoded dictionary.
pub async fn send_extended_handshake<S> (stream: &mut S) -> std::io::Result<()> 
where 
	S: AsyncWrite + Unpin
{
    // Build a bencoded dictionary that advertises our supported extensions.
    // For example, we support "ut_metadata"
	let mut dict: HashMap<String, BValue>  = HashMap::new();

    // "m" maps extension names to their message numbers.
    // We do not choose our own id here; we advertise support for "ut_metadata"
    // and the remote peer will assign an id for it.
	let mut m: HashMap<String, BValue>  = HashMap::new();
	m.insert("ut_metadata".into(), BValue::Integer(20));
	dict.insert("m".into(), BValue::Dict(m));

    // Bencode
	let payload = encode_bvalue(&BValue::Dict(dict));

    // Build the extended message:
    // First byte: Extended message id (we use 20 for handshake)
    // Followed by the bencoded payload.
	let mut message = Vec::with_capacity(1 + payload.len());
    message.push(20); // main message id for extended messages
    message.push(0);  // extended handshake id (0 indicates handshake)
	message.extend_from_slice(&payload);

	let message_len = (message.len() as u32).to_be_bytes();
    stream.write_all(&message_len).await?;
    stream.write_all(&message).await?;
    stream.flush().await?;

	Ok(())
}

/// Receives and validates the BitTorrent handshake. On success, returns the remote peer id.
pub async fn receive_handshake<S>(
    stream: &mut S,
    expected_info_hash: &[u8; 20],
) -> Result<([u8; 20], bool), Error>
where
    S: AsyncRead + Unpin,
{
    let mut buf = [0u8; 68];
    stream.read_exact(&mut buf).await?;
    if buf[0] != BT_PROTOCOL_LEN {
        return Err(Error::new(ErrorKind::Other, "Invalid handshake pstrlen"));
    }
    let pstr_end = 1 + BT_PROTOCOL_LEN as usize;
    if &buf[1..pstr_end] != BT_PROTOCOL_STR.as_bytes() {
        return Err(Error::new(ErrorKind::Other, "Invalid handshake pstr"));
    }

    // Extract reserved bytes.
    let reserved: &[u8] = &buf[pstr_end..pstr_end + 8];
    // Check if the extension bit is set. For example, if reserved[5] has the 0x10 bit set.
    let supports_extensions = (reserved[5] & 0x10) != 0;
    
    // Extract infohash.
    let infohash_start = pstr_end + 8;
    let infohash_end = infohash_start + 20;
    let infohash = &buf[infohash_start..infohash_end];
    if infohash != expected_info_hash {
        return Err(Error::new(ErrorKind::Other, "Infohash mismatch"));
    }

    // Extract peer id.
    let peer_id_start = infohash_end;
    let peer_id_end = peer_id_start + 20;
    let mut peer_id = [0u8; 20];
    peer_id.copy_from_slice(&buf[peer_id_start..peer_id_end]);
    
    Ok((peer_id, supports_extensions))
}

mod tests {
	use super::*;

	#[allow(dead_code)]
	// A helper function to create a valid handshake message.
	fn create_handshake(info_hash: &[u8; 20], peer_id: &[u8; 20]) -> Vec<u8> {
		let mut handshake = Vec::with_capacity(68);
		handshake.push(BT_PROTOCOL_LEN);
		handshake.extend_from_slice(BT_PROTOCOL_STR.as_bytes());
		handshake.extend_from_slice(&[0_u8; 8]); // reserved bytes
		handshake.extend_from_slice(info_hash);
		handshake.extend_from_slice(peer_id);
		handshake
	}

	#[tokio::test]
	async fn test_send_and_receive_handshake() {
		let info_hash = [1u8; 20];
		// let local_peer_id = [2u8; 20];
		let remote_peer_id = [3u8; 20];
		
		// Create a duplex stream to simulate a TCP connection.
		let (mut client, mut server) = tokio::io::duplex(128);

		// Spawn a task to act as the server side, which sends a handshake.
		let server_task = tokio::spawn(async move {
			// Simulate sending a handshake from the "remote" peer.
			let handshake = create_handshake(&info_hash, &remote_peer_id);
			server.write_all(&handshake).await.expect("server write failed");
			server.flush().await.expect("server flush failed");
		});

		// Call the receive function on the client side.
		let (received_peer_id, _exension) = receive_handshake(&mut client, &info_hash)
			.await
			.expect("Handshake validation failed");

		// Ensure that the peer id matches what the server sent.
		assert_eq!(received_peer_id, remote_peer_id);

		server_task.await.unwrap();
	}

	#[tokio::test]
	async fn test_send_handshake() {
		let info_hash = [4u8; 20];
		let peer_id = [5u8; 20];

		// Create a duplex stream to simulate a TCP connection.
		let (mut client, mut server) = tokio::io::duplex(128);

		// Spawn a task to act as the server and read the handshake.
		let server_task = tokio::spawn(async move {
			let mut buf = [0u8; 68];
			server.read_exact(&mut buf).await.expect("server read failed");
			// Verify the handshake fields.

			let mut start: usize = 0;
			// 1-byte: pstrlen (length of the protocol string)
			assert_eq!(buf[start], BT_PROTOCOL_LEN);

			// pstr ("BitTorrent protocol")
			start += 1;
			let mut end = start + BT_PROTOCOL_STR.as_bytes().len();
			assert_eq!(&buf[start..end], BT_PROTOCOL_STR.as_bytes());

			// 8-byte: reserved Exension
			let reserved = [0u8; 8];
			start = start + BT_PROTOCOL_LEN as usize;
			end = start + 8; 
			assert_eq!(&buf[start..end], &reserved);
			
			// info hash
			start += 8;
			end += 20;
			assert_eq!(&buf[start..end], &info_hash);

			start += 20;
			end += 20;
			// 
			assert_eq!(&buf[start..end], &peer_id);
		});

		// Send the handshake from the client side.
		send_handshake(&mut client, &info_hash, &peer_id, false)
			.await
			.expect("send handshake failed");

		server_task.await.unwrap();
	}

	#[tokio::test]
	async fn test_send_handshake_extension() {
		let info_hash = [4u8; 20];
		let peer_id = [5u8; 20];

		// Create a duplex stream to simulate a TCP connection.
		let (mut client, mut server) = tokio::io::duplex(128);

		// Spawn a task to act as the server and read the handshake.
		let server_task = tokio::spawn(async move {
			let mut buf: [u8; 68] = [0u8; 68];
			server.read_exact(&mut buf).await.expect("server read failed");
			// Verify the handshake fields.

			let mut start: usize = 0;
			// 1-byte: pstrlen (length of the protocol string)
			assert_eq!(buf[start], BT_PROTOCOL_LEN);

			// pstr ("BitTorrent protocol")
			start += 1;
			let mut end = start + BT_PROTOCOL_STR.as_bytes().len();
			assert_eq!(&buf[start..end], BT_PROTOCOL_STR.as_bytes());

			// 8-byte: reserved Exension
			let mut reserved = [0u8; 8];
			reserved[5] = 0x10;
			start = start + BT_PROTOCOL_LEN as usize;
			end = start + 8; 
			assert_eq!(&buf[start..end], &reserved);
			
			// 20-byte: info hash
			start += 8;
			end += 20;
			assert_eq!(&buf[start..end], &info_hash);

			// 20-byte: peer id
			start += 20;
			end += 20;
			assert_eq!(&buf[start..end], &peer_id);
		});

		// Send the handshake from the client side.
		send_handshake(&mut client, &info_hash, &peer_id, true)
			.await
			.expect("send handshake failed");

		server_task.await.unwrap();
	}


}