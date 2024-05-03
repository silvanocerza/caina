use core::panic;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{IpAddr, TcpStream},
    path::PathBuf,
};

use bincode::Options;
use clap::Parser;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Deserializer};
use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

/// MetaInfo represents the information found in a .torrent file as defined in BEP003.
/// https://www.bittorrent.org/beps/bep_0003.html#metainfo-files
#[derive(Serialize, Deserialize, Debug)]
struct MetaInfo {
    /// Tracker URL
    announce: String,
    /// Torrent metadata
    info: Info,
}

/// Info represents the metadata found in the `info` field in a .torrent file
/// as defined in BEP003.
/// https://www.bittorrent.org/beps/bep_0003.html#info-dictionary
#[derive(Serialize, Deserialize, Debug)]
struct Info {
    /// UTF-8 encoded string which is the suggested name to save the file (or directory) as.
    /// It is purely advisory.
    name: String,
    /// Number of bytes in each piece the file is split into.
    #[serde(rename = "piece length")]
    piece_length: usize,
    /// Concatenated SHA1 hashes of all pieces.
    /// Splitting this into 20 char long strings will get
    /// all the hashes.
    /// The index of the hash correspond to piece index.
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>,
    /// Length of the file in bytes.
    /// Only present in single file cases.
    /// Must not be present if `files` field is present
    #[serde(default, skip_serializing_if = "Option::is_none")]
    length: Option<usize>,
    /// List of files.
    /// Only present in multi files cases.
    /// Must not be present if `length` field is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    files: Option<Vec<File>>,
}

impl Info {
    fn hash(&self) -> String {
        let buf = match serde_bencode::to_bytes(self) {
            Ok(buf) => buf,
            Err(err) => {
                panic!("Failed parsing: {}", err)
            }
        };
        Sha1::digest(buf).iter().map(|b| format!("{}", b)).collect()
    }

    fn hash_encoded(&self) -> String {
        let buf = match serde_bencode::to_bytes(self) {
            Ok(buf) => buf,
            Err(err) => {
                panic!("Failed parsing: {}", err)
            }
        };
        Sha1::digest(buf)
            .iter()
            .map(|b: &u8| format!("%{:02X}", b))
            .collect()
    }

    fn size(&self) -> usize {
        if self.length.is_some() {
            self.length.expect("This can't happen")
        } else if self.files.is_some() {
            self.files
                .as_ref()
                .expect("This can't happen either")
                .iter()
                .map(|f| f.length)
                .reduce(|acc, l| acc + l)
                .expect("This can't happen")
        } else {
            panic!("Something really bad happened");
        }
    }
}

/// File represents a single file found in the `files` field in a .torrent file
/// as defined in BEP003.
/// https://www.bittorrent.org/beps/bep_0003.html#info-dictionary
#[derive(Serialize, Deserialize, Debug)]
struct File {
    /// Length of the file in bytes
    length: usize,
    ///  A list of UTF-8 encoded strings corresponding to subdirectory names,
    /// the last of which is the actual file name (a zero length list is an error case)
    path: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Peer {
    #[serde(rename = "peer id")]
    id: String,
    ip: String,
    port: String,
}

impl Peer {
    fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

fn deserialize_peers<'de, D>(deserializer: D) -> Result<Vec<Peer>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: serde_bytes::ByteBuf = Deserialize::deserialize(deserializer)?;

    if let Ok(peers) = serde_bencode::from_bytes::<Vec<Peer>>(s.as_slice()) {
        return Ok(peers);
    };

    let mut peers: Vec<Peer> = vec![];
    for chunk in s.chunks(6) {
        let ip: [u8; 4] = chunk[..4].try_into().unwrap();
        let port: [u8; 2] = chunk[4..].try_into().unwrap();

        let ip = IpAddr::from(ip).to_string();
        let port = ((port[0] as u16) << 8 | port[1] as u16).to_string();

        peers.push(Peer {
            id: String::from(""),
            ip,
            port,
        });
    }
    Ok(peers)
}
#[derive(Deserialize, Debug)]
struct TrackerResponse {
    #[serde(rename = "failure reason")]
    failure_reason: Option<String>,
    #[serde(rename = "warning message")]
    warning_message: Option<String>,
    #[serde(default)]
    interval: i32,
    #[serde(default, rename = "min interval")]
    min_interval: Option<i32>,
    #[serde(default, rename = "tracker id")]
    tracker_id: String,
    #[serde(default)]
    complete: i32, // Seeders
    #[serde(default)]
    incomplete: i32, // Leechers
    #[serde(default, deserialize_with = "deserialize_peers")]
    peers: Vec<Peer>,
}

fn generate_peer_id() -> String {
    let random_id_suffix: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(12)
        .map(|b| format!("%{:02X}", b))
        .collect();
    // Use CN000 as prefix, CN from Caina, the name I'll be using for the project
    // 000 is the version, just zeroes for the time being.
    let peer_id_fixed: String = "-CN0000-".bytes().map(|b| format!("%{:02X}", b)).collect();
    format!("{}{}", peer_id_fixed, random_id_suffix)
}

fn build_tracker_url(torrent: &MetaInfo, peer_id: &String) -> String {
    format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&compact={}&left={}",
        torrent.announce,
        torrent.info.hash_encoded(),
        peer_id,
        "6881",
        "0",
        "0",
        "1",
        torrent.info.size(),
    )
}

fn tracker_get(torrent: &MetaInfo, peer_id: &String) -> Result<TrackerResponse, String> {
    if !torrent.announce.starts_with("http") {
        // TODO: Support UDP trackers
        let protocol = torrent.announce.split(":").collect::<Vec<&str>>()[0];
        panic!("{} trackers not supported", protocol)
    }
    let url = build_tracker_url(torrent, &peer_id);
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

#[derive(Serialize, Deserialize, Debug)]
struct Handshake<'a> {
    #[serde(rename = "pstrlen")]
    protocol_string_length: u8,
    #[serde(rename = "pstr")]
    protocol: &'a str,
    #[serde(default, with = "serde_bytes")]
    reserved: [u8; 8],
    #[serde(default)]
    info_hash: &'a str,
    #[serde(default)]
    peer_id: &'a str,
}

impl Handshake<'_> {
    fn new<'a>(info_hash: &'a String, peer_id: &'a String) -> Handshake<'a> {
        Handshake {
            protocol_string_length: 19,
            protocol: "BitTorrent protocol",
            reserved: [0; 8],
            info_hash: info_hash.as_str(),
            peer_id: peer_id.as_str(),
        }
    }
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
