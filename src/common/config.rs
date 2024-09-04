use serde_derive::{Deserialize, Serialize};
use std::{env, fs, path::Path};

pub const CONFIG_FILENAME: &'static str = "config.toml";

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    pub id: String,
    pub ipv4: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub network_adapter: String,
    pub wg_interface: String,
    pub conf_dir: String,
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
        let config_dir = env::var("CONFIG_DIR").unwrap_or_else(|_| "./".to_string());
        let config_path = Path::new(&config_dir).join(CONFIG_FILENAME);
        println!("Loading config from: {:?}", config_path);

        let contents = match fs::read_to_string(config_path) {
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
