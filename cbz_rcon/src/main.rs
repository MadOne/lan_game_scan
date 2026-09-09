use cbz_rcon::RconClient;
use std::io::{self, Write};
use std::net::SocketAddr;

fn prompt(name: &str) -> io::Result<String> {
    let mut line = String::new();

    println!("{name}");
    io::stdout().flush()?;
    io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_string())
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

    let password = match prompt("rcon password: ") {
        Ok(password) => password,
        Err(error) => {
            eprintln!("Error reading password: {error}");
            return;
        }
    };

    let mut client = loop {
        let addr_string = match prompt("ip:port (z.B.: 10.10.1.99:27016): ") {
            Ok(addr) => addr,
            Err(error) => {
                eprintln!("Error reading address: {error}");
                return;
            }
        };

        let addr: SocketAddr = match addr_string.parse() {
            Ok(addr) => addr,
            Err(error) => {
                println!("Invalid address: {error}");
                continue;
            }
        };

        let mut client = RconClient::new(addr, password.clone(), cbz_rcon::RconProtocol::Source);

        match client.connect().await {
            Ok(()) => {
                println!("RCON authentication successful.");
                break client;
            }

            Err(error) => {
                println!("Connection failed: {error}");
            }
        }
    };

    loop {
        let input = match prompt("rcon command: ") {
            Ok(input) => input,
            Err(error) => {
                eprintln!("Error reading command: {error}");
                break;
            }
        };

        if input == "exit" || input == "quit" {
            break;
        }

        match client.command(&input).await {
            Ok(response) => {
                println!("response: {response}");
            }

            Err(error) => {
                println!("RCON error: {error}");
                break;
            }
        }
    }
}
