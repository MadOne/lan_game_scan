use std::{fs, path::PathBuf};

use lan_game_scan::{create_scaner, server::Server, start};
use rusqlite::{Connection, Result};
extern crate directories;
use directories::ProjectDirs;

#[tokio::main]
async fn main() -> Result<()> {
    /*
    // open or create db

    // get OS Path for data
    let path = ProjectDirs::from("com", "mad_ne", "lan_game_scan");
    let mut path = match path {
        Some(val) => val.clone().data_dir().to_owned(),
        None => PathBuf::new(),
    };
    // create the folders if necesarry
    let _bla = fs::create_dir_all(&path);
    path.push("lan_game_scan.db");
    //println!("{path:?}");
    let conn = Connection::open(path)?;

    // if table does not exist, create it
    let _res = conn.execute(
        "Create TABLE lan(
    id INTEGER PRIMARY KEY,
    ip_port STRING,
    hostname STRING,
    game STRING,
    map STRING,
    players INTEGER,
    players_max INTEGER,
    query_port INTEGER,
    rcon STRING)
    ",
        (),
    );
    //println!("{res:?}");

    // if table does not exist, create it
    let _res = conn.execute(
        "Create TABLE favourites(
        id INTEGER PRIMARY KEY,
        ip_port STRING,
        hostname STRING,
        game STRING,
        map STRING,
        players INTEGER,
        players_max INTEGER,
        query_port INTEGER,
        rcon STRING)
        ",
        (),
    );
    //println!("{res:?}");

    let server_receiver = create_scaner().await.clone();
    let mut server_receiver = server_receiver.lock().await;
    loop {
        if let Some(server) = server_receiver.recv().await {
            let ip_port = server.ip_port.clone();
            let mut stmt = conn.prepare("SELECT * FROM lan WHERE ip_port=?1")?;
            let server_iter = stmt.query_map([ip_port.clone()], |row| {
                Ok(Server {
                    ip_port: row.get(1)?,
                    hostname: row.get(2)?,
                    game: row.get(3)?,
                    map: row.get(4)?,
                    players: row.get(5)?,
                    players_max: row.get(6)?,
                    query_port: row.get(7)?,
                    rcon: row.get(8)?,
                })
            })?;

            if server_iter.count() == 0 {
                // add server
                conn.execute("INSERT INTO lan (id, ip_port, hostname, game, map, players, players_max, query_port) VALUES(NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", ("NULL", server.ip_port, server.hostname, server.game, server.map, server.players, server.players_max, server.query_port))?;
            } else {
                // update server
                let mut stmt = conn.prepare("UPDATE lan SET hostname = ?1, game=?2, map=?3, players=?4, players_max=?5, query_port=?6 WHERE ip_port=?7")?;
                let _bla = stmt.execute((
                    server.hostname,
                    server.game,
                    server.map,
                    server.players,
                    server.players_max,
                    server.query_port,
                    ip_port,
                ))?;
            }
        }
    }
    */
    start().await;
    loop {}
    Ok(())
}
