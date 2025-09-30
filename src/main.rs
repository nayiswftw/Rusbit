
mod bencode;
mod torrent;
mod tracker;
mod peer;
mod utils;
mod engine;
mod message;
mod piece_manager;
mod piece_queue;
mod magnet;
mod file_io;
use std::env;
use log::error;

use crate::engine::use_command;
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 2 {
        error!("Usage:\n  {} decode <bencoded_string>\n  {} info <file.torrent>", 
                  args[0], args[1]);
        return;
    }

	let res = use_command(args);
	eprintln!("{:?}", res.err());
}
