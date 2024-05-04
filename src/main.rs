use core::panic;

pub mod message;
pub mod peer_id;
pub mod torrentfile;

use crate::peer_id::generate_peer_id;
use crate::torrentfile::MetaInfo;

use std::{collections::HashMap, path::PathBuf};

use clap::Parser;

#[derive(Parser)]
struct Cli {
    torrent_file: PathBuf,
}

fn main() {
    let args = Cli::parse();
    let torrent_file = args.torrent_file;

    let peer_id = generate_peer_id();

    // Read the file
    let buf = match std::fs::read(&torrent_file) {
        Ok(buf) => buf,
        Err(err) => {
            panic!("{}", err);
        }
    };

    // Deserialize torrent data
    let res = match serde_bencode::from_bytes::<MetaInfo>(&buf.as_slice()) {
        Ok(res) => res,
        Err(err) => {
            panic!("{}", err);
        }
    };

    // Print torrent info
    println!("{}", res.info.name);
    println!("Tracker: {}", res.announce);
    println!("Piece length: {}", res.info.piece_length);

    println!("Number of pieces: {}", res.info.pieces.chunks(20).len());

    if let Some(size) = res.info.length {
        println!("File bytes: {}", size);
    }
    if let Some(files) = &res.info.files {
        println!("Files:");
        for file in files {
            println!("  {}", file.path.join("/"));
            println!("    File bytes: {}", file.length);
        }
    }

    let tracker_response = match res.tracker_get(&peer_id) {
        Ok(res) => res,
        Err(err) => panic!("{}", err),
    };

    let mut streams = HashMap::new();
    for peer in tracker_response.peers {
        println!("Trying to connect to {}", peer.address());
        let stream = match peer.open_stream(&res.info.hash(), &peer_id) {
            Ok(s) => s,
            Err(err) => {
                println!("Couldn't connect with peer: {:?}", err);
                continue;
            }
        };
        streams.insert(peer.address(), stream);

        println!("Opened stream with {}", peer.address());
    }
}
