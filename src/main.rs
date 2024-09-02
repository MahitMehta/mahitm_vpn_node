mod common;
mod utils;

use common::config::Config;
use futures_util::{stream::StreamExt, SinkExt};
use log::{debug, error, info};
use serde_derive::{Deserialize, Serialize};
use std::{process::exit, sync::RwLock, time::Duration};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

#[derive(Deserialize, Debug)]
pub enum ENodeMessage {
    CreatePeer = 0,
    RemovePeer = 1,
    CreatePeerResponse = 2,
    RemovePeerResponse = 3,
    RequestTunnel = 4,
    RequestTunnelResponse = 5,
}

impl From<u8> for ENodeMessage {
    fn from(v: u8) -> Self {
        match v {
            0 => ENodeMessage::CreatePeer,
            1 => ENodeMessage::RemovePeer,
            2 => ENodeMessage::CreatePeerResponse,
            3 => ENodeMessage::RemovePeerResponse,
            4 => ENodeMessage::RequestTunnel,
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
    #[serde(rename = "privateKey")]
    pub private_key: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "dstPort")]
    pub dst_port: u16,
    #[serde(rename = "srcPort")]
    pub src_port: u16
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

#[derive(Debug)]
pub struct State {
    wg_private_key: String,
    wg_public_key: String,
}

struct NodeWebSocket {
    config: Config,
    state: RwLock<State>,
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl NodeWebSocket {
    async fn new(config: Config) -> Self {
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

        info!("Connecting to {}", url);
        let (stream, _) = connect_async(url)
            .await
            .expect("Failed to Connect to Control Plane");

        info!("Connected to Control Plane");

        let wg_private_key = match utils::generate_private_key() {
            Ok(k) => k,
            Err(e) => {
                error!("{}", e);
                exit(1);
            }
        };
        let wg_public_key = match utils::generate_public_key(&wg_private_key) {
            Ok(k) => k,
            Err(e) => {
                error!("{}", e);
                exit(1);
            }
        };

        Self {
            state: RwLock::new(State {
                wg_private_key,
                wg_public_key
            }),
            config,
            stream
        }
    }

    async fn create_peer(&mut self, create_peer: NodeMessage<CreatePeer>) {
        debug!("Create Peer: {:?}", create_peer);

        // TODO: Create Peer on Wireguard Server

        let create_peer_res = NodeMessage::<CreatePeerResponse> {
            r#type: ENodeMessage::CreatePeerResponse as u8,
            body: CreatePeerResponse {
                ipv4: "".to_string(),
                private_key: "".to_string(),
                public_key: "".to_string(),
                user_id: create_peer.body.user_id,
            },
        };
        self.stream
            .send(Message::Text(
                serde_json::to_string(&create_peer_res).unwrap(),
            ))
            .await
            .unwrap();
    }

    async fn handle_node_message(&mut self, msg: Message) {
        let node_msg: NodeMessageType = serde_json::from_str(&msg.to_string()).unwrap();
        match ENodeMessage::from(node_msg.r#type) {
            ENodeMessage::CreatePeer => {
                let create_peer: NodeMessage<CreatePeer> =
                    serde_json::from_str(&msg.to_string()).unwrap();
                self.create_peer(create_peer).await;
            }
            ENodeMessage::RemovePeer => {
                let remove_peer: NodeMessage<CreatePeer> =
                    serde_json::from_str(&msg.to_string()).unwrap();
                debug!("Remove Peer: {:?}", remove_peer);

                // TODO: Remove Peer on Wireguard Server
            }
            _ => {
                error!("Invalid Packet Type");
            }
        }
    }

    async fn request_tunnel(&mut self) {
        self.stream.send(Message::Text(
            serde_json::to_string(&NodeMessage::<RequestTunnel> {
                r#type: ENodeMessage::RequestTunnel as u8,
                body: RequestTunnel {
                    ipv4: self.config.node.ipv4.clone(),
                    src_port: self.config.node.src_port,
                    dst_port: self.config.node.dst_port,
                    private_key: self.state.read().unwrap().wg_private_key.clone(),
                    public_key: self.state.read().unwrap().wg_public_key.clone(),
                },
            })
            .unwrap(),
        )).await.unwrap();
    }

    async fn run(&mut self) {
        self.request_tunnel().await; 

        let mut interval = tokio::time::interval(Duration::from_millis(1000));
        loop {
            tokio::select! {
                msg = self.stream.next() => {
                    if let Some(msg) = msg {
                        if let Ok(msg) = msg {
                            if msg.is_pong() {
                                debug!("Received Pong");
                                continue;
                            }
    
                            if !msg.is_text() {
                                continue;
                            }
    
                            self.handle_node_message(msg).await;
                        }
                    }
                }
                _ = interval.tick() => {
                    self.stream.send(Message::Ping(vec![])).await.unwrap();
                }
            }
        }
    }


}


#[tokio::main]
async fn main() {
    pretty_env_logger::init_timed();

    let config = Config::load().unwrap_or_else(|e| {
        error!("{}", e);
        exit(1);
    });    

    let mut node_ws = NodeWebSocket::new(config).await;
    node_ws.run().await;
}
