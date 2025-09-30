use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum BValue {
	ByteString(Vec<u8>), // raw bytes for any string
	Integer(i64),
	List(Vec<BValue>),
	Dict(HashMap<String, BValue>) // keys are always UTF-8 in .torrent
}

