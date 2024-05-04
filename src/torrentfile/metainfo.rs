use crate::torrentfile::Info;
use serde_derive::{Deserialize, Serialize};

/// MetaInfo represents the information found in a .torrent file as defined in BEP003.
/// https://www.bittorrent.org/beps/bep_0003.html#metainfo-files
#[derive(Serialize, Deserialize, Debug)]
pub struct MetaInfo {
    /// Tracker URL
    pub announce: String,
    /// Torrent metadata
    pub info: Info,
}
