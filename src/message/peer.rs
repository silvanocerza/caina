use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{IpAddr, TcpStream},
    path::PathBuf,
};

use bincode::Options;
use clap::Parser;
use serde::{Deserialize, Deserializer};
use serde_derive::{Deserialize, Serialize};

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
}
