use std::num::ParseIntError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BencodeError {
    #[error("Unexpected end of input")]
	UnexpectedEnd,

	#[error("Invalid Integer {0}")]
	InvalidInteger(String),

	#[error("Invalid Format {0}")]
	InvalidFormat(String),

	#[error("Parse error {0}")]
	ParseError(ParseIntError)
}

impl From<ParseIntError> for BencodeError  {
		fn from(err: ParseIntError) -> Self {
			BencodeError::ParseError(err)
		}

}