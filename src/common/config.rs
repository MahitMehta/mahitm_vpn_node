use serde_derive::{Deserialize, Serialize};
use std::fs;

pub const CONFIG_FILENAME: &'static str = "config.toml";

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    pub id: String,
    pub ipv4: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub network_adapter: String,
    pub wg_interface: String,
    pub conf_path: String,
    pub private_key: Option<String>,
    pub public_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ControlPlane {
    pub host: String,
    pub secure: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub node: Node,
    pub control_plane: ControlPlane,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let contents = match fs::read_to_string(CONFIG_FILENAME) {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("Unable to retrieve Node config: {e}"));
            }
        };

        let config = match toml::from_str::<Config>(&contents) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{}", e);
                return Err(format!("Unable to parse Node config: {e}"));
            }
        };

        Ok(config)
    }

    pub fn update_keys(&mut self, private_key: String, public_key: String) {
        self.node.private_key = Some(private_key);
        self.node.public_key = Some(public_key);

        let contents = toml::to_string(&self).unwrap();
        fs::write(CONFIG_FILENAME, contents).unwrap();
    }
}
