use std::{collections::HashMap, net::SocketAddr};

use dioxus::prelude::*;

use crate::state::GameServer;

// Pop $number bytes from vector.
// When number == 0 pop a 0 terminated string

fn get_save_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "android")]
    {
        let config_dir = std::path::PathBuf::from("/data/user/0/com.madone.lan_game_scan/files");

        if let Err(error) = std::fs::create_dir_all(&config_dir) {
            tracing::error!(
                "Could not create Android app data directory {}: {}",
                config_dir.display(),
                error
            );
            return None;
        }

        Some(config_dir.join("favorites.json"))
    }

    #[cfg(not(target_os = "android"))]
    {
        let Some(proj_dirs) = directories::ProjectDirs::from("com", "madone", "serverbrowser")
        else {
            tracing::error!("Could not find application config directory");
            return None;
        };

        let config_dir = proj_dirs.config_dir();

        if let Err(error) = std::fs::create_dir_all(config_dir) {
            tracing::error!(
                "Could not create config directory {}: {}",
                config_dir.display(),
                error
            );
            return None;
        }

        Some(config_dir.join("favorites.json"))
    }
}

pub fn save_to_disk(favs: &HashMap<SocketAddr, GameServer>) {
    let Some(path) = get_save_path() else {
        return;
    };

    let json = match serde_json::to_string_pretty(favs) {
        Ok(json) => json,
        Err(error) => {
            tracing::error!("Could not serialize favorites: {}", error);
            return;
        }
    };

    if let Err(error) = std::fs::write(&path, json) {
        tracing::error!(
            "Could not write favorites file {}: {}",
            path.display(),
            error
        );
    }
}

pub fn load_from_disk() -> HashMap<SocketAddr, GameServer> {
    let Some(path) = get_save_path() else {
        return HashMap::new();
    };

    let data = match std::fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return HashMap::new();
        }
        Err(error) => {
            tracing::warn!(
                "Could not read favorites file {}: {}",
                path.display(),
                error
            );
            return HashMap::new();
        }
    };

    match serde_json::from_str(&data) {
        Ok(map) => map,
        Err(error) => {
            tracing::warn!(
                "Could not parse favorites file {}: {}",
                path.display(),
                error
            );
            HashMap::new()
        }
    }
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
