use crate::message::Peer;
use std::net::IpAddr;

use serde::{Deserialize, Deserializer};
use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TrackerResponse {
    #[serde(rename = "failure reason")]
    pub failure_reason: Option<String>,
    #[serde(rename = "warning message")]
    pub warning_message: Option<String>,
    #[serde(default)]
    pub interval: i32,
    #[serde(default, rename = "min interval")]
    pub min_interval: Option<i32>,
    #[serde(default, rename = "tracker id")]
    pub tracker_id: String,
    #[serde(default)]
    pub complete: i32, // Seeders
    #[serde(default)]
    pub incomplete: i32, // Leechers
    #[serde(default, deserialize_with = "deserialize_peers")]
    pub peers: Vec<Peer>,
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
