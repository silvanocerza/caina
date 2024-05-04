use serde_derive::Deserialize;

use crate::message::Handshake;

use std::{
    io::{Read, Write},
    net::TcpStream,
};

use bincode::Options;

#[derive(Deserialize, Debug)]
pub struct Peer {
    #[serde(rename = "peer id")]
    pub id: String,
    pub ip: String,
    pub port: String,
}

impl Peer {
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    pub fn open_stream(&self, info_hash: &String, peer_id: &String) -> Result<TcpStream, String> {
        // let timeout = Duration::new(3, 0);
        // let mut stream = match TcpStream::connect_timeout(peer.address(), timeout) {
        let mut stream = match TcpStream::connect(self.address()) {
            Ok(s) => s,
            Err(err) => return Err(format!("Failed opening TCP stream: {}", err)),
        };

        // Handshake
        let handshake = Handshake::new(info_hash, peer_id);

        // let mut data: Vec<u8> = vec![];
        // data.push(handshake.protocol_string_length);
        // data.extend_from_slice(handshake.protocol.as_bytes());
        // data.extend_from_slice(&handshake.reserved[..]);
        // data.extend_from_slice(handshake.info_hash.as_bytes());
        // data.extend_from_slice(handshake.peer_id.as_bytes());

        let options = bincode::DefaultOptions::new();
        // .with_big_endian()
        // .allow_trailing_bytes()
        // .with_fixint_encoding();

        let handshake = match options.serialize(&handshake) {
            Ok(b) => b,
            Err(err) => return Err(format!("Failed serializing handshake: {}", err)),
        };

        match stream.write(&handshake) {
            Ok(size) => println!("Sent {} bytes", size),
            Err(err) => return Err(format!("Failed sending handshake: {}", err)),
        };

        // We're using 68 as the size here cause we know the exact length of the handshake message, that is:
        // protocol_string_length: 1 bytes
        // protocol: 19 bytes
        // reserved: 8 bytes
        // info_hash: 20 bytes
        // peer_id: 20 bytes
        // This is not completely reliable as we relying on the fact the protocol is string is exactly
        // 19 bytes long, but it could not be. This is good enough for the time being.
        let mut buf = [0; 300];

        match stream.read(&mut buf[..]) {
            Ok(size) => println!("Received {} bytes", size),
            Err(err) => return Err(format!("Failed reading data from peer: {}", err)),
        };

        println!("sent     bytes: {:?}", handshake);
        println!("received bytes: {:?}", buf);

        let handshake: Handshake = match options.deserialize(&buf) {
            Ok(res) => res,
            Err(err) => return Err(format!("Failed deserializing handshake: {}", err)),
        };
        if handshake.info_hash != info_hash {
            // This is not the file we want, there's something wrong.
            // Close the connection.
            _ = stream.shutdown(std::net::Shutdown::Both);
            return Err(String::from("Received wrong info hash from peer"));
        }
        if peer_id.len() > 0 && peer_id != handshake.peer_id {
            // This peer is returning a different id than expected.
            // Close the connection.
            _ = stream.shutdown(std::net::Shutdown::Both);
            return Err(String::from("Received unexpected peer id from peer"));
        }
        Ok(stream)
    }
}
