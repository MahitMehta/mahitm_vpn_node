use serde_derive::Deserialize;
use std::fs;

pub const CONFIG_FILENAME: &'static str = "config.toml";

#[derive(Debug, Deserialize)]
pub struct Node {
    pub id: String,
    pub ipv4: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub network_adapter: String,
    pub conf_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ControlPlane {
    pub host: String,
    pub secure: bool,
}

#[derive(Debug, Deserialize)]
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
}
