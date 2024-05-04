use crate::message::TrackerResponse;
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
    pub fn build_tracker_url(&self, peer_id: &Vec<u8>) -> String {
        format!(
            "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&compact={}&left={}",
            self.announce, // Tracker url
            self.info
                .hash()
                .iter()
                .map(|b| format!("%{:02X}", b))
                .collect::<String>(), // info_hash
            peer_id
                .iter()
                .map(|b| format!("%{:02X}", b))
                .collect::<String>(), // peer_id
            "6881",        // port
            "0",           // uploaded
            "0",           // downloaded
            "1",           // compact
            self.info.size(), // left
        )
    }

    pub fn tracker_get(&self, peer_id: &Vec<u8>) -> Result<TrackerResponse, String> {
        if !self.announce.starts_with("http") {
            // TODO: Support UDP trackers
            let protocol = self.announce.split(":").collect::<Vec<&str>>()[0];
            panic!("{} trackers not supported", protocol)
        }
        let url = self.build_tracker_url(&peer_id);
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
}
