use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::Read,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::bencode::{decode_bencode, BValue};
use crate::torrent::calculate_info_hash_from_struct;

/// Represents a .torrent file, including the announce URL and the associated info.
#[derive(Serialize, Deserialize)]
pub struct Torrent {
    pub announce: String,       // The tracker URL
    pub info: TorrentInfo,      // Torrent metadata
    pub info_hash: [u8; 20],      // Infohash 
}

/// Contains detailed metadata about the torrent's content.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TorrentInfo {
    pub length: usize,          // Total size of the file(s)
    pub name: String,           // Name of the file or folder
    pub piece_length: usize,    // Size of each piece
    pub pieces: Vec<[u8; 20]>,    // SHA-1 hashes are 20 bytes each
}

impl Torrent {
    /// Attempts to read a .torrent file from disk and parse its contents.
    ///
    /// Returns a boxed error if the file cannot be opened, read, or the bencoded
    /// structure cannot be properly parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut file = File::open(path)
            .map_err(|e| format!("I/O error while opening file: {}", e))?;
        let mut buf = Vec::new();

        // Read the file as raw bytes
        file.read_to_end(&mut buf)
            .map_err(|e| format!("I/O error while reading file: {}", e))?;

        // Decode the bencoded data
        let (_consumed, bvalue) = decode_bencode(&buf)
            .map_err(|e| format!("Bencode error: {:?}", e))?;

        // Convert the BValue structure to a Torrent
        Self::from_bvalue(&bvalue)
    }

    /// Creates a `Torrent` from a `BValue` (the result of a bencode parse).
    ///
    /// Returns a boxed error if the required fields are missing or invalid.
    pub fn from_bvalue(value: &BValue) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let root_dict = match value {
            BValue::Dict(m) => m,
            _ => return Err("Root of .torrent must be a dictionary".into()),
        };

        // Assume get_bytestring and the others are already converted to use Box<dyn Error + Send + Sync>
        let announce: String = get_bytestring(&root_dict, "announce")?;


        let value = root_dict
            .get("info")
            .ok_or_else(|| "Missing 'info' dictionary".to_string())?;

        let info_dict = match value {
            BValue::Dict(m) => m,
            _ => return Err("'info' must be a dictionary".into()),
        };

        let info: TorrentInfo = TorrentInfo::from_bvalue(&info_dict)?;
        let info_hash = calculate_info_hash_from_struct(&info);

        Ok(Torrent {
            announce,
            info,
            info_hash,
        })
    }
}

impl TorrentInfo {
    pub fn from_bvalue(info_dict: &HashMap<String, BValue>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let name: String = get_bytestring(&info_dict, "name")?;
        let length = get_integer(&info_dict, "length")?;
        let piece_length = get_integer(&info_dict, "piece length")?;
        let pieces_bytes = lookup_bytestring(&info_dict, "pieces")?;

        // Chunk the pieces bytes into 20-byte pieces.
        let pieces = pieces_bytes
            .chunks_exact(20)
            .map(|chunk| {
                let mut hash = [0u8; 20];
                hash.copy_from_slice(chunk);
                hash
            })
            .collect();

        Ok(TorrentInfo {
            name,
            length,
            piece_length,
            pieces,
        })
    }
}


/// Looks up a key in the dictionary and returns a byte slice if the value is a ByteString.
/// Returns a boxed error if the key is missing or the value is of the wrong type.
pub fn lookup_bytestring<'a>(
    dict: &'a HashMap<String, BValue>, 
    key: &str,
) -> Result<&'a [u8], Box<dyn Error + Send + Sync>> {
    // Try to get the value; if missing, return an error.
    let val = dict
        .get(key)
        .ok_or_else(|| format!("Missing '{}'", key))?;
    
    // Check if the value is the expected type.
    match val {
        BValue::ByteString(b) => Ok(b),
        _ => Err(format!("'{}' must be ByteString", key).into()),
    }
}

/// Gets a ByteString from the dictionary and converts it into a UTF-8 String.
/// Returns a boxed error if the key is missing, not a ByteString, or the bytes are not valid UTF-8.
pub fn get_bytestring(
    dict: &HashMap<String, BValue>,
    key: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    // Use lookup_bytestring to get the raw bytes.
    let bytes = lookup_bytestring(dict, key)?;
    // Convert the bytes to a UTF-8 string.
    let result = String::from_utf8(bytes.to_vec())
        .map_err(|_| format!("'{}' value not valid UTF-8", key))?;
    Ok(result)
}

/// Retrieves an integer value from the dictionary.
/// Returns a boxed error if the key is missing or if the value is not an Integer.
pub fn get_integer(
    dict: &HashMap<String, BValue>,
    key: &str,
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    let val = dict
        .get(key)
        .ok_or_else(|| format!("Missing '{}'", key))?;
    
    match val {
        BValue::Integer(b) => Ok(*b as usize),
        _ => Err(format!("'{}' must be a Number", key).into()),
    }
}