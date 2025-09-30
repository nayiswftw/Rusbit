// src/file_io.rs
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter, AsyncSeekExt, SeekFrom};
use std::path::Path;
use std::io::Error;

pub async fn write_piece_to_file_at_offset(
	piece_data: &[u8],
	piece_index: u32,
	output_path: &str,
	piece_length: u32,
	full_file: bool,

) -> Result<(), Error> {
	let file_path = Path::new(output_path);
	// Open the file in read/write mode; you may want to truncate or create it.
	let file = OpenOptions::new()
		.create(true)
		.write(true)
		.open(file_path)
		.await?;
	let mut writer = BufWriter::new(file);

	// Compute the file offset for this piece.
	let offset = if full_file {
		 piece_index as u64 * piece_length as u64
	} else {
		0 as u64
	};

	writer.seek(SeekFrom::Start(offset)).await?;
	writer.write_all(piece_data).await?;
	writer.flush().await?;
	
	println!("Piece {} written to file at offset {}", piece_index, offset);
	Ok(())
}
