
use serde_json::{Value, json};
use hex;
use super::BValue;

/// Encode a `BValue` back into a bencoded `Vec<u8>`.
pub fn encode_bvalue(value: &BValue) -> Vec<u8> {
	let mut out: Vec<u8> = Vec::new();

	match value {
		BValue::Integer(i) => {
			out.extend_from_slice(b"i");
			out.extend_from_slice(i.to_string().as_bytes());
			out.extend_from_slice(b"e");
		}
		BValue::ByteString(bytes) => {
			out.extend_from_slice(bytes.len().to_string().as_bytes());
			out.push(b':');
			out.extend_from_slice(bytes);
		}
		BValue::List(items) => {
			out.push(b'l');
			for item in items {
				let encoded = encode_bvalue(item);
				out.extend_from_slice(&encoded);
			}
			out.push(b'e');
		}
		BValue::Dict(dict) => {
            out.push(b'd');
			let mut sorted_keys: Vec<&String> = dict.keys().collect();
			sorted_keys.sort();
			for key in sorted_keys {
				out.extend_from_slice(key.len().to_string().as_bytes());
				out.push(b':');
				out.extend_from_slice(key.as_bytes());
				let encoded_val = encode_bvalue(&dict[key]);
				out.extend_from_slice(&encoded_val);
			}
			out.push(b'e');
		}
	}
	out
}

/// Convert a `BValue` into JSON (using Serde JSON `Value`).
/// 
/// - `Integer(i)` => JSON number
/// - `ByteString(bytes)` => Attempt UTF-8; if invalid, store hex in `\"_bytes_hex\"`.
/// - `List(...)` => JSON array
/// - `Dict(...)` => JSON object
pub fn bvalue_to_json(bv: &BValue) -> Value {
	match bv {
        BValue::Integer(i) => json!(i),

        BValue::ByteString(bytes) => {
            // Attempt to interpret as UTF-8
            match String::from_utf8(bytes.clone()) {
                Ok(utf8_str) => {
                    // If it's valid UTF-8, just a normal JSON string
                    Value::String(utf8_str)
                }
                Err(_) => {
                    // Otherwise, store as hex or base64 or something else
                    // We'll use hex here:
                    json!({ "_bytes_hex": hex::encode(bytes) })
                }
            }
        }
		BValue::List(list_items) => {
            // Convert each element of the Vec<BValue> into JSON
            let json_items: Vec<Value> = list_items.iter()
                .map(|item| bvalue_to_json(item))
                .collect();
            Value::Array(json_items)
        }

        BValue::Dict(map) => {
            // Convert each (String -> BValue) pair into a JSON key -> Value
            let mut json_map = serde_json::Map::new();
            for (k, v) in map {
                json_map.insert(k.clone(), bvalue_to_json(v));
            }
            Value::Object(json_map)
        }
	}

}

