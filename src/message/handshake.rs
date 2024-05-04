use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Handshake<'a> {
    #[serde(rename = "pstrlen")]
    pub protocol_string_length: u8,
    #[serde(rename = "pstr")]
    pub protocol: &'a str,
    #[serde(default, with = "serde_bytes")]
    pub reserved: [u8; 8],
    #[serde(default)]
    pub info_hash: &'a str,
    #[serde(default)]
    pub peer_id: &'a str,
}

impl Handshake<'_> {
    pub fn new<'a>(info_hash: &'a String, peer_id: &'a String) -> Handshake<'a> {
        Handshake {
            protocol_string_length: 19,
            protocol: "BitTorrent protocol",
            reserved: [0; 8],
            info_hash: info_hash.as_str(),
            peer_id: peer_id.as_str(),
        }
    }
}
