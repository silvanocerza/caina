use core::panic;
use std::{net::IpAddr, path::PathBuf};

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
        Sha1::digest(buf)
            .iter()
            .map(|b| format!("%{:02X}", b))
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

fn build_tracker_url(torrent: &MetaInfo) -> String {
    format!(
        "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&compact={}&left={}",
        torrent.announce,
        torrent.info.hash(),
        generate_peer_id(),
        "6881",
        "0",
        "0",
        "0",
        torrent.info.size(),
    )
}

fn tracker_get(torrent: &MetaInfo) -> Result<TrackerResponse, String> {
    if !torrent.announce.starts_with("http") {
        // TODO: Support UDP trackers
        let protocol = torrent.announce.split(":").collect::<Vec<&str>>()[0];
        panic!("{} trackers not supported", protocol)
    }
    let url = build_tracker_url(torrent);
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

#[derive(Parser)]
struct Cli {
    torrent_file: PathBuf,
}

fn main() {
    let args = Cli::parse();
    let torrent_file = args.torrent_file;
    // let torrent_file = "ubuntu-23.10.1-desktop-amd64.iso.torrent";

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

    let tracker_response = match tracker_get(&res) {
        Ok(res) => res,
        Err(err) => panic!("{}", err),
    };
}
