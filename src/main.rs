use core::panic;

pub mod message;
pub mod peer_id;
pub mod torrentfile;

use crate::message::{Handshake, Peer, TrackerResponse};
use crate::peer_id::generate_peer_id;
use crate::torrentfile::MetaInfo;

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
};

use bincode::Options;
use clap::Parser;

fn tracker_get(torrent: &MetaInfo, peer_id: &String) -> Result<TrackerResponse, String> {
    if !torrent.announce.starts_with("http") {
        // TODO: Support UDP trackers
        let protocol = torrent.announce.split(":").collect::<Vec<&str>>()[0];
        panic!("{} trackers not supported", protocol)
    }
    let url = torrent.build_tracker_url(&peer_id);
    let client = reqwest::blocking::Client::new();
    let res = match client.get(url).send() {
        Ok(res) => res,
        Err(err) => return Err(format!("Failed request: {}", err)),
    };

    let body = match res.bytes() {
        Ok(body) => body,
        Err(err) => return Err(format!("Failed reading response body: {}", err)),
    };

    let tracker_res = match serde_bencode::from_bytes::<TrackerResponse>(&body) {
        Ok(tracker_res) => tracker_res,
        Err(err) => return Err(format!("Failed parsing response body: {}", err)),
    };

    match tracker_res.failure_reason {
        Some(err) => return Err(err),
        _ => {}
    }

    Ok(tracker_res)
}

fn open_stream(peer: &Peer, info_hash: &String, peer_id: &String) -> Result<TcpStream, String> {
    // let timeout = Duration::new(3, 0);
    // let mut stream = match TcpStream::connect_timeout(peer.address(), timeout) {
    let mut stream = match TcpStream::connect(peer.address()) {
        Ok(s) => s,
        Err(err) => return Err(format!("Failed opening TCP stream: {}", err)),
    };

    // Handshake
    let handshake = Handshake::new(info_hash, peer_id);

    // let mut data: Vec<u8> = vec![];
    // data.push(handshake.protocol_string_length);
    // data.extend_from_slice(handshake.protocol.as_bytes());
    // data.extend_from_slice(&handshake.reserved[..]);
    // data.extend_from_slice(handshake.info_hash.as_bytes());
    // data.extend_from_slice(handshake.peer_id.as_bytes());

    let options = bincode::DefaultOptions::new();
    // .with_big_endian()
    // .allow_trailing_bytes()
    // .with_fixint_encoding();

    let handshake = match options.serialize(&handshake) {
        Ok(b) => b,
        Err(err) => return Err(format!("Failed serializing handshake: {}", err)),
    };

    match stream.write(&handshake) {
        Ok(size) => println!("Sent {} bytes", size),
        Err(err) => return Err(format!("Failed sending handshake: {}", err)),
    };

    // We're using 68 as the size here cause we know the exact length of the handshake message, that is:
    // protocol_string_length: 1 bytes
    // protocol: 19 bytes
    // reserved: 8 bytes
    // info_hash: 20 bytes
    // peer_id: 20 bytes
    // This is not completely reliable as we relying on the fact the protocol is string is exactly
    // 19 bytes long, but it could not be. This is good enough for the time being.
    let mut buf = [0; 300];

    match stream.read(&mut buf[..]) {
        Ok(size) => println!("Received {} bytes", size),
        Err(err) => return Err(format!("Failed reading data from peer: {}", err)),
    };

    println!("sent     bytes: {:?}", handshake);
    println!("received bytes: {:?}", buf);

    let handshake: Handshake = match options.deserialize(&buf) {
        Ok(res) => res,
        Err(err) => return Err(format!("Failed deserializing handshake: {}", err)),
    };
    if handshake.info_hash != info_hash {
        // This is not the file we want, there's something wrong.
        // Close the connection.
        _ = stream.shutdown(std::net::Shutdown::Both);
        return Err(String::from("Received wrong info hash from peer"));
    }
    if peer_id.len() > 0 && peer_id != handshake.peer_id {
        // This peer is returning a different id than expected.
        // Close the connection.
        _ = stream.shutdown(std::net::Shutdown::Both);
        return Err(String::from("Received unexpected peer id from peer"));
    }
    Ok(stream)
}

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

    let tracker_response = match tracker_get(&res, &peer_id) {
        Ok(res) => res,
        Err(err) => panic!("{}", err),
    };

    let mut streams = HashMap::new();
    for peer in tracker_response.peers {
        println!("Trying to connect to {}", peer.address());
        let stream = match open_stream(&peer, &res.info.hash(), &peer_id) {
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
