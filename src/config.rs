use serde::{Deserialize, Serialize};

use std::collections::HashSet;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
pub struct DiscordConfig {
    pub token: String,
    pub prefix: String,
    pub allowed_channels: HashSet<u64>,
    pub owners: HashSet<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HttpConfig {
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WebSocketConfig {
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub discord: DiscordConfig,
    pub http: HttpConfig,
    pub websocket: WebSocketConfig,
    pub oauth2: OAuth2Config,
}

const CONFIG_FILE_NAME: &str = "config.toml";

pub fn load_config() -> std::io::Result<Config> {
    let path = Path::new(CONFIG_FILE_NAME);

    if !Path::exists(path) {
        let default = Config::default();
        std::fs::write(path, toml::to_string(&default).unwrap().as_bytes())?;
    }

    let string = std::fs::read_to_string(path)?;

    Ok(toml::from_str(&*string).unwrap())
}

impl Default for Config {
    fn default() -> Self {
        Config {
            discord: DiscordConfig {
                token: String::from("CHANGEME"),
                prefix: String::from("$"),
                allowed_channels: HashSet::new(),
                owners: HashSet::new(),
            },
            http: HttpConfig {
                address: String::from("127.0.0.1:3000"),
            },
            websocket: WebSocketConfig {
                address: String::from("127.0.0.1:3001"),
            },
            oauth2: OAuth2Config {
                client_id: String::from("CHANGEME"),
                client_secret: String::from("CHANGEME"),
                redirect_url: String::from("CHANGEME"),
            },
        }
    }
}
