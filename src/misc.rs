use std::{collections::HashMap, net::SocketAddr};

use dioxus::prelude::*;

use crate::scanner::GameServer;

// Pop $number bytes from vector.
// When number == 0 pop a 0 terminated string

fn get_save_path() -> std::path::PathBuf {
    #[cfg(target_os = "android")]
    {
        let config_dir = std::path::PathBuf::from("/data/user/0/com.madone.lan_game_scan/files");

        std::fs::create_dir_all(&config_dir).expect("Could not create Android app data directory");

        return config_dir.join("favorites.json");
    }

    #[cfg(not(target_os = "android"))]
    {
        let proj_dirs = directories::ProjectDirs::from("com", "madone", "serverbrowser")
            .expect("Could not find config directory");

        let config_dir = proj_dirs.config_dir();

        std::fs::create_dir_all(config_dir).expect("Could not create config directory");

        config_dir.join("favorites.json")
    }
}

pub fn save_to_disk(favs: &HashMap<SocketAddr, GameServer>) {
    let path = get_save_path();
    if let Ok(json) = serde_json::to_string_pretty(favs) {
        let _ = std::fs::write(path, json);
    }
}

pub fn load_from_disk() -> HashMap<SocketAddr, GameServer> {
    let path = get_save_path();
    if let Ok(data) = std::fs::read_to_string(path) {
        if let Ok(map) = serde_json::from_str(&data) {
            return map;
        }
    }
    HashMap::new()
}

pub async fn connect_to_server(ip_and_port: String, password: String) {
    use std::process::Command;
    let args = format!("steam://connect/{}/{}", ip_and_port, password);
    let _ = Command::new("steam").args([args]).output();
}

#[derive(PartialEq, Props, Clone)]
pub struct Props {
    pub game_server: GameServer,
}
