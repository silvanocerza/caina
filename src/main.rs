use core::panic;
use std::path::PathBuf;

use clap::Parser;
use serde_bytes::ByteBuf;
use serde_derive::{Deserialize, Serialize};

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
    pieces: ByteBuf,
    /// Length of the file in bytes.
    /// Only present in single file cases.
    /// Must not be present if `files` field is present
    #[serde(default)]
    length: Option<usize>,
    /// List of files.
    /// Only present in multi files cases.
    /// Must not be present if `length` field is present.
    #[serde(default)]
    files: Option<Vec<File>>,
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

#[derive(Parser)]
struct Cli {
    torrent_file: PathBuf,
}

fn main() {
    let args = Cli::parse();

    // Read the file
    let buf = match std::fs::read(&args.torrent_file) {
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
    if let Some(files) = res.info.files {
        println!("Files:");
        for file in files {
            println!("  {}", file.path.join("/"));
            println!("    File bytes: {}", file.length);
        }
    }
}
