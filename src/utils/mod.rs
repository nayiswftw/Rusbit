mod url_encode;

pub use url_encode::{url_encode_bytes, url_decode};
use rand::Rng;

pub fn generate_peer_id() -> [u8; 20] {
	let mut rng = rand::thread_rng();
	let mut peer_id = [0u8; 20];
	rng.fill(&mut peer_id);
	peer_id
}