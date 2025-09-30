use std::collections::HashMap;
use super::error::BencodeError;
use crate::bencode::bvalue::BValue;

pub fn decode_bencode(input: &[u8]) -> Result<(usize, BValue), BencodeError> {
    if input.is_empty() {
        return Err(BencodeError::UnexpectedEnd);
    }

    match input[0] {
        b'i' => decode_integer(input),
        b'l' => decode_list(input),
        b'd' => decode_dict(input),
        c if c.is_ascii_digit() => decode_string(input),
        c => Err(BencodeError::InvalidFormat(format!(
            "Unexpected byte: {}",
            c
        ))),
    }
}

fn decode_integer(input: &[u8]) -> Result<(usize, BValue), BencodeError> {
	if input[0] != b'i' {
		return Err(BencodeError::InvalidFormat(format!(
			"Integer must start with 'i', {:?}, {:?}", 
			&input[0], b'i'
		)));
	}
	let end_pos = input
		.iter()
		.position(|&b| b == b'e')
		.ok_or_else(|| BencodeError::InvalidFormat("Missing 'e' for integer".to_string()))?;

	// Parse integer between 'i' and 'e'
	let num_str = std::str::from_utf8(&input[1..end_pos])
		.map_err(|_| BencodeError::InvalidFormat("Non-UTF-8 data in integer".to_string()))?;

	// Leading Zeros no allowed
	if num_str.starts_with('0') && num_str.len() > 1 &&  !num_str.starts_with("-0") {
        return Err(BencodeError::InvalidInteger(format!(
            "Leading zeros are not allowed: {}",
            num_str
        )));
	}

    // convert to i64
    let parsed = num_str.parse::<i64>()
		.map_err(|e| {
			BencodeError::InvalidInteger(format!("Failed to parse integer '{}': {}", num_str, e))
	})?;

	// add 1 to account for 'e'
    Ok((end_pos + 1, BValue::Integer(parsed)))

}


/// Decodes a Bencoded string of the form `<length>:<items>e`.
fn decode_string(encoded: &[u8]) -> Result<(usize, BValue), BencodeError> {
    // The string length is found by reading digits up until ':'.
    // Example: "5:hello"

	let colon_index = encoded
		.iter()
		.position(|&b| b == b':')
		.ok_or_else(|| BencodeError::InvalidFormat("Missing ':' in string".to_string()))?;

	let str_length = std::str::from_utf8(&encoded[..colon_index])
		.map_err(|e| BencodeError::InvalidFormat(
			format!("Ivalid UTF-8 in string length: {}", e)))?;

	let length = str_length.parse::<usize>()
		.map_err(|e| BencodeError::InvalidFormat(
			format!("Invalid String Length:{} err: {}", str_length, e)))?;

	let start_data = colon_index + 1;
	let end_data = start_data + length;

    if end_data > encoded.len() {
        return Err(BencodeError::UnexpectedEnd);
    }

	let data = &encoded[start_data..end_data];
    Ok((end_data, BValue::ByteString(data.to_vec())))
}

/// Decodes a Bencoded list of the form `l<items>e`.
fn decode_list(encoded: &[u8]) -> Result<(usize, BValue), BencodeError> {
	if encoded[0] != b'l' {
		return Err(BencodeError::InvalidFormat(format!(
			"List must start with 'l' {} {}",
			&encoded[0], b'l')
		));
	}

    let mut idx = 1; // skip 'l'
    let mut items = Vec::new();

    while idx < encoded.len() && encoded[idx] != b'e' {
        let (consumed, val) = decode_bencode(&encoded[idx..])?;
        idx += consumed;
        items.push(val);
    }

    // If we've run out of input, the list is unclosed
    if idx >= encoded.len() {
        return Err(BencodeError::InvalidFormat(
            "Unclosed list (missing 'e')".to_string(),
        ));
    }

	// add 1 to account for 'e'
    Ok((idx + 1, BValue::List(items)))
}

fn decode_dict(encoded: &[u8]) -> Result<(usize, BValue), BencodeError> {
    if encoded[0] != b'd' {
		return Err(BencodeError::InvalidFormat(format!(
			"Dict must start with 'd' {} {}",
			&encoded[0], b'd')
		));
    }

    let mut idx = 1; // Skip the initial 'd'
    let mut map = HashMap::new();

    // Loop until we reach 'e' or run out of input
    while idx < encoded.len() && encoded[idx] != b'e' {

        // Decode a key (must be a bencoded string)
        let (key_length, key_value) = decode_string(&encoded[idx..])?;
        idx += key_length;
        // Dictionary keys must be strings
        let key_str = match key_value {
            BValue::ByteString(bytes) => {
				String::from_utf8(bytes).map_err(|_| {
					BencodeError::InvalidFormat("Dict key not valid UTF-8".to_string())
				})?
			}
            _ => {
                return Err(BencodeError::InvalidFormat(
                    "Dict key must be a ByteString".to_string(),
                ));
            }
        };

        // Decode the value (can be int, string, list, or dict)
        let (consumed_val, value) = decode_bencode(&encoded[idx..])?;
        idx += consumed_val;

        map.insert(key_str, value);
    }

    // Here, either we ran out of input or we encountered an 'e'
    // If we've run out of input, it's an unclosed dictionary
    if idx >= encoded.len() {
        return Err(BencodeError::InvalidFormat(
            "Unclosed dictionary (missing 'e')".to_string(),
        ));
    }

	// add 1 to account for 'e'
    Ok((idx + 1, BValue::Dict(map)))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_integer() {
        let input = b"i42e";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());
        assert_eq!(value, BValue::Integer(42));
    }

    #[test]
    fn test_decode_negative_integer() {
        let input = b"i-13e";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());
        assert_eq!(value, BValue::Integer(-13));
    }

    #[test]
    fn test_decode_integer_zero() {
        let input = b"i0e";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());
        assert_eq!(value, BValue::Integer(0));
    }

    #[test]
    fn test_decode_string() {
        let input = b"5:hello";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());
        assert_eq!(value, BValue::ByteString("hello".as_bytes().to_vec()));
    }

    #[test]
    fn test_decode_empty_string() {
        let input = b"0:";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());
        assert_eq!(value, BValue::ByteString("".as_bytes().to_vec()));
    }

    #[test]
    fn test_decode_list() {
        // l4:spami42ee => ["spam", 42]
        let input = b"l4:spami42ee";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());
        assert_eq!(
            value,
            BValue::List(vec![BValue::ByteString("spam".as_bytes().to_vec()), BValue::Integer(42)])
        );
    }

	#[test]
	fn test_decode_nested_list() {
		// l4:spaml4:eggi3eeee => ["spam", ["egg", 3]]
		let input = b"l4:spaml3:eggi3eee";
		let (consumed, value) = decode_bencode(input).unwrap();
		assert_eq!(consumed, input.len());
		assert_eq!(
			value,
			BValue::List(vec![
				BValue::ByteString("spam".as_bytes().to_vec()),
				BValue::List(vec![
					BValue::ByteString("egg".as_bytes().to_vec()),
					BValue::Integer(3)
				]),
			])
		);
	}
	

    #[test]
    fn test_decode_dict() {
        // d3:bar4:spam3:fooi42ee => {"bar":"spam", "foo":42}
        let input = b"d3:bar4:spam3:fooi42ee";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());
        // Construct an equivalent serde_json::Value for comparison
        let mut expected_map = HashMap::new();
        expected_map.insert("bar".to_string(), BValue::ByteString("spam".as_bytes().to_vec()));
        expected_map.insert("foo".to_string(), BValue::Integer(42));
        let expected = BValue::Dict(expected_map);
        assert_eq!(value, expected);
    }

    #[test]
    fn test_decode_empty_dict() {
        // de => {}
        let input = b"de";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());
        let expected = BValue::Dict(HashMap::new());
        assert_eq!(value, expected);
    }

    #[test]
    fn test_decode_dict_with_nested_list() {
        // d3:foo l4:spami1ee 3:bar4:eggse
        // => {"foo": ["spam", 1], "bar": "eggs"}
        let input = b"d3:fool4:spami1ee3:bar4:eggse";
        let (consumed, value) = decode_bencode(input).unwrap();
        assert_eq!(consumed, input.len());

        let mut expected_map = HashMap::new();
        expected_map.insert(
            "foo".to_string(),
            BValue::List(vec![
                BValue::ByteString("spam".as_bytes().to_vec()),
                BValue::Integer(1.into()),
            ]),
        );
        expected_map.insert("bar".to_string(), BValue::ByteString("eggs".as_bytes().to_vec()));
        assert_eq!(value, BValue::Dict(expected_map));
    }

    //
    // Malformed Inputs: Test expected failures
    //

    #[test]
    fn test_decode_integer_missing_e() {
        let input = b"i42";
        let result = decode_bencode(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_leading_zeros() {
        // 0123 is invalid unless it's just 0 or -0
        let input = b"i0123e";
        let result = decode_bencode(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_string_missing_colon() {
        let input = b"5hello"; // missing colon
        let result = decode_bencode(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_list_unclosed() {
        let input = b"l4:spam";
        let result = decode_bencode(input);
	    assert!(result.is_err());
    }

    #[test]
    fn test_decode_dict_unclosed() {
        let input = b"d3:foo4:spam";
        let result = decode_bencode(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_dict_key_not_string() {
        // Suppose we try "i42e" as a key
        // d i42e 4:spam e => malformed, dictionary keys must be strings
        let input = b"di42e4:spame";
        let result = decode_bencode(input);
        assert!(result.is_err());
    }
}