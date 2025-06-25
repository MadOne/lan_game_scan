use lan_game_scan::start;

#[tokio::main]
async fn main() {
    start().await;
    loop {}
}
