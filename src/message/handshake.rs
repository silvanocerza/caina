pub struct Handshake {
    pub protocol_string_length: u8,
    pub protocol: String,
    pub reserved: [u8; 8],
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>,
}

impl Handshake {
    pub fn new(info_hash: &Vec<u8>, peer_id: &Vec<u8>) -> Handshake {
        Handshake {
            protocol_string_length: 19,
            protocol: String::from("BitTorrent protocol"),
            reserved: [0; 8],
            info_hash: info_hash.clone(),
            peer_id: peer_id.clone(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data: Vec<u8> = vec![];
        data.push(self.protocol_string_length);
        data.extend_from_slice(self.protocol.as_bytes());
        data.extend_from_slice(&self.reserved[..]);
        data.extend(&self.info_hash);
        data.extend(&self.peer_id);
        data
    }

    pub fn from_bytes(data: &[u8; 68]) -> Result<Handshake, String> {
        let protocol_string_length = data[0];
        let protocol =
            match String::from_utf8(data[1..(protocol_string_length + 1) as usize].to_vec()) {
                Ok(s) => s.to_string(),
                Err(err) => return Err(format!("Failed parsing protocol: {}", err)),
            };
        let mut reserved = [0; 8];
        for i in 0..7 {
            reserved[i] = data[(protocol_string_length + 1) as usize + i + 1];
        }
        let info_hash_start = (protocol_string_length + 1 + 8) as usize;
        let info_hash_end = info_hash_start + 20;
        let info_hash = data[info_hash_start..info_hash_end].to_vec();

        let peer_id_start = info_hash_end;
        let peer_id = data[peer_id_start..].to_vec();
        Ok(Handshake {
            protocol_string_length,
            protocol,
            reserved,
            info_hash,
            peer_id,
        })
    }
}
