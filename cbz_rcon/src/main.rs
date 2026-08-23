use cbz_rcon::RconClient;
use std::io::{self, Write};
use std::net::SocketAddr;

fn prompt(name: &str) -> String {
    let mut line = String::new();

    print!("{}", name);
    io::stdout().flush().unwrap();

    io::stdin()
        .read_line(&mut line)
        .expect("Error: Could not read a line");

    line.trim().to_string()
}

#[tokio::main]
async fn main() {
    println!(
        r"
     _____ ____ ______    _____   _____ ____  _   _
    / ____|  _ \___  /   |  __ \ / ____/ __ \| \ | |
   | |    | |_) | / /    | |__) | |   | |  | |  \| |
   | |    |  _ < / /     |  _  /| |   | |  | | . ` |
   | |____| |_) / /__    | | \ \| |___| |__| | |\  |
    \_____|____/_____|   |_|  \_\\_____\____/|_| \_|
    "
    );

    let password = prompt("rcon password: ");

    let mut client = loop {
        let addr_string = prompt("ip:port (z.B.: 10.10.1.99:27016): ");

        let addr: SocketAddr = match addr_string.parse() {
            Ok(addr) => addr,
            Err(error) => {
                println!("Invalid address: {}", error);
                continue;
            }
        };

        let mut client = RconClient::new(addr, password.clone());

        match client.connect().await {
            Ok(()) => {
                println!("RCON authentication successful.");
                break client;
            }

            Err(error) => {
                println!("Connection failed: {}", error);
            }
        }
    };

    loop {
        let input = prompt("rcon command: ");

        if input == "exit" || input == "quit" {
            break;
        }

        match client.command(&input).await {
            Ok(response) => {
                println!("response: {}", response);
            }

            Err(error) => {
                println!("RCON error: {}", error);
                break;
            }
        }
    }
}
