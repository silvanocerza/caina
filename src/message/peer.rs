use serde_derive::Deserialize;

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
