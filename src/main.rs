mod common;
mod socket;
mod utils;

use common::config::Config;
use log::error;
use socket::NodeWebSocket;
use std::process::exit;

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

    let mut node_ws = NodeWebSocket::new(&url, config).await;
    node_ws.run_wrapper(&url).await;
}
