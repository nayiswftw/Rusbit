
use crate::magnet::error::MagnetError;
use crate::utils::url_decode;
use core::str;
use std::collections::HashMap;

pub fn decode_magnet(input: &str) -> Result<HashMap<String, String>, MagnetError> {
    if input.is_empty() {
        return Err(MagnetError::UnexpectedEnd);
    }

	if input.starts_with("magnet:") {
        // The magnet URI should be like "magnet:?xt=urn:btih:..."
        let rest = &input[7..]; // remove "magnet:"

        // Look for the '?' that starts the query parameters.
		if let Some(q_pos) = rest.chars().into_iter().position(|b| b == '?') {
			return decode_magnet(&rest[q_pos..]);
		} else {
			return Err(MagnetError::InvalidFormat("Missing '?' in magnet URI".into()));
		}		
	} 

    // Now expect the input to start with '?' or '&', and contain an '='.
	if !input.starts_with('?') {
        return Err(MagnetError::InvalidFormat(
            "Magnet parameters must start with '?'".into(),
		));
	};

	let mut result_map: HashMap<String, String> = HashMap::new();

	let params: Vec<(String, String)> = decode_magnet_parameters(&input[1..])?;
	for (key, value) in params {
		match key.as_str() {
			"xt" => { 
				let info_hash = decode_info_hash(&value)?;
				result_map.insert("info_hash".to_string(), info_hash);
			 }
			"dn" => {
				result_map.insert("file_name".to_string(), value.to_string());
			},
			"tr" => {
				let value = url_decode(&value);
				result_map.insert("announce".to_string(), value.to_string());
			}
			_ => {
				result_map.insert(key, value);
			}
		}
	} 
	
	Ok(result_map)
}

fn decode_magnet_parameters(input: &str) -> Result<Vec<(String, String)>, MagnetError> {

	let mut params = Vec::new();

	for param in input.split('&') {
		if param.is_empty() {
			continue;
		}

		let mut split = param.splitn(2, '=');

		let key = split.next().ok_or_else(||
			MagnetError::InvalidFormat("Missing key in magnet parameter".to_string())
		)?;
		let value = split.next().ok_or_else(||
			MagnetError::InvalidFormat("Missing value in magnet parameter".to_string())
		)?;

		params.push((key.to_string(), value.to_string()));
	}
	Ok(params)
}

fn decode_info_hash(input: &str)-> Result<String, MagnetError> {
    let prefix = "urn:btih:";
	if !input.starts_with(prefix) {
        return Err(MagnetError::InvalidFormat( "urn:btih missing".into()));
    }
	let hash_str  = &input[prefix.len()..];
	Ok(hash_str.to_string())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_magnet() {
        let input = "magnet:?xt=urn:btih:c5fb9894bdaba464811b088d806bdd611ba490a&dn=magnet1.gif&tr=http%3A%2F%2Fbittorrent-test-tracker.codecrafters.io%2Fannounce";
        let res = decode_magnet(input).unwrap();
		let info_hash = res.get("info_hash").unwrap();
		let announce = res.get("announce").unwrap();
		let file_name = res.get("file_name").unwrap();

		assert_eq!(info_hash, "c5fb9894bdaba464811b088d806bdd611ba490a");
        assert_eq!(announce, "http://bittorrent-test-tracker.codecrafters.io/announce");
        assert_eq!(file_name, "magnet1.gif");
    }
}