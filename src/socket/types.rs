use std::fmt::{self, Display, Formatter};

use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub enum ENodeMessage {
    CreatePeer = 0,
    RemovePeer = 1,
    CreatePeerResponse = 2,
    RemovePeerResponse = 3,
    RequestTunnel = 4,
    RequestTunnelResponse = 5,
}

impl Into<u8> for ENodeMessage {
    fn into(self) -> u8 {
        self as u8
    }
}

impl From<u8> for ENodeMessage {
    fn from(v: u8) -> Self {
        match v {
            0 => ENodeMessage::CreatePeer,
            1 => ENodeMessage::RemovePeer,
            2 => ENodeMessage::CreatePeerResponse,
            3 => ENodeMessage::RemovePeerResponse,
            4 => ENodeMessage::RequestTunnel,
            5 => ENodeMessage::RequestTunnelResponse,
            _ => panic!("Invalid Packet Type"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CreatePeer {
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Serialize, Debug)]
pub struct CreatePeerResponse {
    pub ipv4: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Serialize, Debug)]
pub struct RemovePeerResponse {
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Serialize, Debug)]
pub struct RequestTunnel {
    pub ipv4: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "dstPort")]
    pub dst_port: u16,
    #[serde(rename = "srcPort")]
    pub src_port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Peer {
    pub ipv4: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Deserialize, Debug)]
pub struct RequestTunnelResponse {
    #[serde(rename = "ipv4")]
    pub ipv4: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    pub peers: Vec<Peer>,
}

#[derive(Deserialize, Debug)]
pub struct RemovePeer {
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Deserialize, Debug)]
pub struct NodeMessageType {
    pub r#type: u8,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NodeMessage<T> {
    pub r#type: u8,
    pub body: T,
}
#[derive(Serialize, Debug)]
pub struct WireguardInterface {
    #[serde(rename = "PrivateKey")]
    pub private_key: String,
    #[serde(rename = "Address")]
    pub address: String,
    #[serde(rename = "ListenPort")]
    pub listen_port: u16,
    #[serde(rename = "PostUp")]
    pub post_up: String,
}

#[derive(Serialize, Debug)]
pub struct WireguardConf {
    #[serde(rename = "Interface")]
    pub(crate) interface: WireguardInterface,
}

impl Display for WireguardConf {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut wg_conf = toml::to_string(&self).unwrap();
        let conf_prefix =
            "# Note: Do not edit this file directly.\n# Your changes will be overwritten!";
        wg_conf = wg_conf.replace('"', "");
        write!(f, "{}\n\n{}", conf_prefix, wg_conf)
    }
}
