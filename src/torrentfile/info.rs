use serde_derive::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

/// Info represents the metadata found in the `info` field in a .torrent file
/// as defined in BEP003.
/// https://www.bittorrent.org/beps/bep_0003.html#info-dictionary
#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
    /// UTF-8 encoded string which is the suggested name to save the file (or directory) as.
    /// It is purely advisory.
    pub name: String,
    /// Number of bytes in each piece the file is split into.
    #[serde(rename = "piece length")]
    pub piece_length: usize,
    /// Concatenated SHA1 hashes of all pieces.
    /// Splitting this into 20 char long strings will get
    /// all the hashes.
    /// The index of the hash correspond to piece index.
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
    /// Length of the file in bytes.
    /// Only present in single file cases.
    /// Must not be present if `files` field is present
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
    /// List of files.
    /// Only present in multi files cases.
    /// Must not be present if `length` field is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<File>>,
}

impl Info {
    pub fn hash(&self) -> String {
        let buf = match serde_bencode::to_bytes(self) {
            Ok(buf) => buf,
            Err(err) => {
                panic!("Failed parsing: {}", err)
            }
        };
        Sha1::digest(buf).iter().map(|b| format!("{}", b)).collect()
    }

    pub fn hash_encoded(&self) -> String {
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

    pub fn size(&self) -> usize {
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
pub struct File {
    /// Length of the file in bytes
    pub length: usize,
    ///  A list of UTF-8 encoded strings corresponding to subdirectory names,
    /// the last of which is the actual file name (a zero length list is an error case)
    pub path: Vec<String>,
}
