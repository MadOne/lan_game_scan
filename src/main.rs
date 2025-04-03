use lan_game_scan::ServerScaner;

#[tokio::main]
async fn main() {
    //let (_sender,mut reciever) = start_daemon().await;
    //let mut newmap: BTreeMap<String, Server> = BTreeMap::new();
    //while let Some(server) = reciever.recv().await {
        //println!("Recieved {} from {}", bytes.len(), addr);
        //lan_game_scan::process_response(bytes, addr, sender);
    //    println!("{server:?}");
    //    newmap.insert(server.addr.to_string(), server);
    //    println!("ServerMap: {newmap:?}");
        
    //}
    let mut a = ServerScaner::new();
    a.start_daemon().await;
    loop {
        a.print_servers();
        
    }

}



