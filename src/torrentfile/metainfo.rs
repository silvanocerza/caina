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

impl MetaInfo {
    pub fn build_tracker_url(&self, peer_id: &String) -> String {
        format!(
            "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&compact={}&left={}",
            self.announce,
            self.info.hash_encoded(),
            peer_id,
            "6881",
            "0",
            "0",
            "1",
            self.info.size(),
        )
    }
}
