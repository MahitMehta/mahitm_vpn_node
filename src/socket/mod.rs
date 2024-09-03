mod types;

use futures_util::{stream::StreamExt, SinkExt};
use log::{debug, error, info, warn};
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::{
    collections::HashMap,
    fs::File,
    process::{exit, Stdio},
    time::Duration,
};
use tokio::{net::TcpStream, process::Command};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use types::{
    CreatePeer, CreatePeerResponse, ENodeMessage, NodeMessage, NodeMessageType, Peer, RemovePeer,
    RemovePeerResponse, RequestTunnel, RequestTunnelResponse, WireguardConf, WireguardInterface,
};

use crate::common::config::Config;
use crate::utils;

#[derive(Debug)]
pub struct State {
    active: bool,
    peers: HashMap<String, Peer>,
}

pub struct NodeWebSocket {
    config: Config,
    state: State,
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl NodeWebSocket {
    pub async fn new(url: &String, mut config: Config) -> Self {
        info!("Connecting to {}", url);
        let (stream, _) = connect_async(url)
            .await
            .expect("Failed to Connect to Control Plane");

        info!("Connected to Control Plane");

        if config.node.private_key.is_none() || config.node.public_key.is_none() {
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
            config.update_keys(wg_private_key, wg_public_key);
        }

        Self {
            state: State {
                active: false,
                peers: HashMap::new(),
            },
            config,
            stream,
        }
    }

    async fn reconnect(&mut self, url: &String) -> Result<(), Box<dyn std::error::Error>> {
        tokio::time::sleep(Duration::from_secs(1)).await;

        error!("Attempting Reconnection to Control Plane");
        self.state.active = false;

        let (stream, _) = connect_async(url).await?;
        self.stream = stream;

        Ok(())
    }

    async fn create_peer(&mut self, create_peer: CreatePeer) {
        debug!("Create Peer: {:?}", create_peer);

        let mut n = 2u8;
        let ipv4 = loop {
            let ipv4 = format!("10.8.0.{}", n);
            if !self.state.peers.contains_key(&ipv4) {
                break ipv4;
            }
            if n == 255 {
                error!("No more available IPs");
                return;
            }
            n += 1;
        };

        let private_key = match utils::generate_private_key() {
            Ok(k) => k,
            Err(e) => {
                error!("Failed to Generate Peer Private Key: {}", e);
                return;
            }
        };
        let public_key = match utils::generate_public_key(&private_key) {
            Ok(k) => k,
            Err(e) => {
                error!("Failed to Generate Peer Public Key: {}", e);
                return;
            }
        };

        match utils::add_peer_to_conf(&self.config.node.wg_interface, &ipv4, &public_key) {
            Ok(_) => {
                info!(
                    "Peer @{ipv4} added to Wireguard Config, user_id: {}",
                    create_peer.user_id
                );
            }
            Err(e) => {
                error!("Failed to add Peer to Wireguard Config: {}", e);
                return;
            }
        }

        // TODO: Validate if peer was actually added
        self.state.peers.insert(
            ipv4.clone(),
            Peer {
                ipv4: ipv4.clone(),
                private_key: private_key.clone(),
                public_key: public_key.clone(),
                user_id: create_peer.user_id.clone(),
            },
        );

        let create_peer_res = NodeMessage::<CreatePeerResponse> {
            r#type: ENodeMessage::CreatePeerResponse.into(),
            body: CreatePeerResponse {
                ipv4,
                private_key,
                public_key,
                user_id: create_peer.user_id,
            },
        };
        self.stream
            .send(Message::Text(
                serde_json::to_string(&create_peer_res).unwrap(),
            ))
            .await
            .unwrap();
    }

    async fn remove_peer(&mut self, remove_peer: RemovePeer) {
        debug!("Remove Peer: {:?}", remove_peer);

        let peer = self
            .state
            .peers
            .iter()
            .find(|(_, peer)| peer.user_id == remove_peer.user_id);
        if peer.is_none() {
            error!("Peer not found: {}", remove_peer.user_id);
            return;
        }
        let peer = peer.unwrap().1;
        match utils::remove_peer_from_conf(
            &self.config.node.wg_interface,
            &peer.ipv4,
            &peer.public_key,
        ) {
            Ok(_) => {
                info!(
                    "Peer @{} removed from Wireguard Config, user_id: {}",
                    peer.ipv4, peer.user_id
                );
                self.state.peers.remove(&peer.ipv4.clone());
            }
            Err(e) => {
                error!("Failed to remove Peer from Wireguard Config: {}", e);
                self.state.peers.remove(&peer.ipv4.clone());
                return;
            }
        }

        let remove_peer_res = NodeMessage::<RemovePeerResponse> {
            r#type: ENodeMessage::RemovePeerResponse.into(),
            body: RemovePeerResponse {
                user_id: remove_peer.user_id,
            },
        };
        self.stream
            .send(Message::Text(
                serde_json::to_string(&remove_peer_res).unwrap(),
            ))
            .await
            .unwrap();
    }

    async fn tunnel_setup(&mut self, request_tunnel_res: RequestTunnelResponse) {
        self.state.active = true;

        request_tunnel_res.peers.iter().for_each(|peer| {
            self.state.peers.insert(peer.ipv4.clone(), peer.clone());
        });

        let post_up = format!(
            "iptables -t nat -A POSTROUTING -s 10.8.0.0/24 -o {} -j MASQUERADE; iptables -A INPUT -p udp -m udp --dport {} -j ACCEPT; iptables -A FORWARD -i {} -j ACCEPT; iptables -A FORWARD -o {} -j ACCEPT;",
            self.config.node.network_adapter,
            self.config.node.src_port,
            self.config.node.wg_interface,
            self.config.node.wg_interface
        );

        let wg_conf = WireguardConf {
            interface: WireguardInterface {
                private_key: self.config.node.private_key.clone().unwrap(),
                address: format!("{}/24", "10.8.0.1"),
                listen_port: self.config.node.src_port,
                post_up,
            },
        };

        let path = format!(
            "{}/{}.conf",
            self.config.node.conf_path, self.config.node.wg_interface
        );

        // TODO: Replace with OpenOptions
        match File::create(&path) {
            Ok(mut output) => {
                write!(output, "{}", wg_conf).expect("Failed to update wg conf");
            }
            Err(e) => {
                error!("Failed to open Wireguard Config: {}", e);
            }
        }

        Command::new("wg-quick")
            .args(["down", path.as_str()])
            .output()
            .await
            .expect("Failed to shutdown wg interface");

        let output = Command::new("wg-quick")
            .args(["up", path.as_str()])
            .stdout(Stdio::piped())
            .output()
            .await
            .expect("Failed to start wg interface");

        let stderr = String::from_utf8(output.stderr).unwrap();
        if stderr.len() > 0 {
            warn!("Error while starting wg interface: {}", stderr);
        }
    }

    async fn handle_node_message(&mut self, msg: Message) {
        let node_msg: NodeMessageType = serde_json::from_str(&msg.to_string()).unwrap();
        match ENodeMessage::from(node_msg.r#type) {
            ENodeMessage::RequestTunnelResponse => {
                let request_tunnel_res: NodeMessage<RequestTunnelResponse> =
                    serde_json::from_str(&msg.to_string()).unwrap();
                debug!("Request Tunnel Response");
                self.tunnel_setup(request_tunnel_res.body).await;
            }
            ENodeMessage::CreatePeer => {
                let create_peer: NodeMessage<CreatePeer> =
                    serde_json::from_str(&msg.to_string()).unwrap();
                if !self.state.active {
                    error!(
                        "Tunnel inactive, cannot create peer: {}",
                        create_peer.body.user_id
                    );
                    return;
                }
                self.create_peer(create_peer.body).await;
            }
            ENodeMessage::RemovePeer => {
                let remove_peer: NodeMessage<RemovePeer> =
                    serde_json::from_str(&msg.to_string()).unwrap();
                if !self.state.active {
                    error!(
                        "Tunnel inactive, cannot remove peer: {}",
                        remove_peer.body.user_id
                    );
                    return;
                }
                self.remove_peer(remove_peer.body).await;
            }
            _ => {
                error!("Invalid Packet Type");
            }
        }
    }

    async fn request_tunnel(&mut self) {
        self.stream
            .send(Message::Text(
                serde_json::to_string(&NodeMessage::<RequestTunnel> {
                    r#type: ENodeMessage::RequestTunnel.into(),
                    body: RequestTunnel {
                        ipv4: self.config.node.ipv4.clone(),
                        src_port: self.config.node.src_port,
                        dst_port: self.config.node.dst_port,
                        public_key: self.config.node.public_key.clone().unwrap(),
                    },
                })
                .unwrap(),
            ))
            .await
            .unwrap();
    }

    pub fn run_wrapper<'a>(
        &'a mut self,
        url: &'a String,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(async move {
            match self.run().await {
                Ok(_) => {}
                Err(e) => {
                    error!("Run Wrapper Error: {}", e);

                    for _ in 0..5 {
                        match self.reconnect(url).await {
                            Ok(_) => {
                                info!("Reconnected to Control Plane");
                                let _ = Box::pin(self.run_wrapper(url).await);
                                break;
                            }
                            Err(e) => {
                                error!("Reconnection Error: {}", e);
                            }
                        }
                    }

                    error!("Reconnection Attempts Exhausted, Exiting");
                    exit(1);
                }
            }
        })
    }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
                    match self.stream.send(Message::Ping(vec![])).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed to send Ping");
                            return Err(Box::new(e));
                        }
                    }
                }
            }
        }
    }
}
