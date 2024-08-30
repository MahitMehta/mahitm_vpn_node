mod common;

use common::config::Config;
use futures_util::{stream::StreamExt, SinkExt};
use log::{debug, error};
use serde_derive::Deserialize;
use std::process::exit;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Deserialize, Debug)]
pub enum ENodeMessage {
    CreatePeer = 0,
    RemovePeer = 1
}


impl From<u8> for ENodeMessage {
    fn from(v: u8) -> Self {
        match v {
            0 => ENodeMessage::CreatePeer,
            1 => ENodeMessage::RemovePeer,
            _ => panic!("Invalid Packet Type"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CreatePeer {
    pub user_id: String
}

#[derive(Deserialize, Debug)]
pub struct NodeMessageType {
    pub r#type: ENodeMessage,
}

#[derive(Deserialize, Debug)]
pub struct NodeMessage<T> {
    pub r#type: ENodeMessage,
    pub body: T

}

#[tokio::main]
async fn main() {
    pretty_env_logger::init_timed();

    let config = Config::load().unwrap_or_else(|e| {
        error!("{}", e);
        exit(1);
    });

    let url = format!(
        "{}://{}/node?id={}",
        if config.control_plane.secure {
            "wss"
        } else {
            "ws"
        },
        config.control_plane.host,
        config.node.id
    );
    debug!("Connecting to {}", url);
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to Connect to Control Plane");

    debug!("Connected to Control Plane");
    ws_stream.send(Message::Ping(vec![])).await.unwrap();
    while let Some(msg) = ws_stream.next().await {
        let msg = msg.unwrap();

        if msg.is_pong() {
            debug!("Received Pong");
            continue;
        }

        if !msg.is_text() { continue }
        
        let node_msg: NodeMessageType = serde_json::from_str(&msg.to_string()).unwrap();
        match node_msg.r#type {
            ENodeMessage::CreatePeer => {
                let create_peer: NodeMessage<CreatePeer> = serde_json::from_str(&msg.to_string()).unwrap();
                debug!("Create Peer: {:?}", create_peer);
            }
            ENodeMessage::RemovePeer => {
                let remove_peer: NodeMessage<CreatePeer> = serde_json::from_str(&msg.to_string()).unwrap();
                debug!("Remove Peer: {:?}", remove_peer);
            }
        }
    }
}
