use core::panic;
use std::path::PathBuf;

use clap::Parser;
use rand::{distributions::Alphanumeric, Rng};
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

fn generate_peer_id() -> String {
    let random_id_suffix: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(12)
        .map(|b| format!("%{:02X}", b))
        .collect();
    // Use CC000 as prefix, CC from Cascata, the name I'll be using for the project
    // 000 is the version, just zeroes for the time being.
    let peer_id_fixed: String = "-CC0000-".bytes().map(|b| format!("%{:02X}", b)).collect();
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

fn tracker_get(torrent: &MetaInfo) {
    let url = build_tracker_url(torrent);
    let client = reqwest::blocking::Client::new();
    let res = match client.get(url).send() {
        Ok(res) => res,
        Err(err) => panic!("Failed request: {}", err),
    };
    let body = match res.text() {
        Ok(body) => body,
        Err(err) => panic!("Failed reading response body: {}", err),
    };

    println!("body = {body:?}");
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

    tracker_get(&res);
}
