// -----------------------------------------------------------------------------
// main.rs
// -----------------------------------------------------------------------------

use live_log::{
    http_catcher::LiveLog,
    parser::{LogEvent, LogType},
};

#[tokio::main]
async fn main() {
    let visible_logs = [
        LogType::Chat,
        LogType::Connection,
        LogType::RoundWin,
        LogType::GameOver,
        // Add the specific connection variants here if they are represented
        // by separate LogEvent variants. Otherwise LogType::Connection
        // covers all connection events.
    ];

    let mut live_log = LiveLog::new().await.expect("Failed to start LiveLog");

    println!(
        "\x1b[1;36mLiveLog listening on port {}\x1b[0m",
        live_log.port()
    );

    let mut receiver = live_log.take_receiver();

    while let Some(parsed) = receiver.recv().await {
        if !visible_logs.contains(&parsed.log_type) {
            continue;
        }

        if !parsed.pretty.is_empty() {
            println!(
                "\x1b[1m[{}]\x1b[0m {}",
                parsed.log_type.label(),
                parsed.pretty
            );
        } else if !matches!(parsed.event, LogEvent::Ignored) {
            println!("\x1b[1;33m[UNMATCHED]\x1b[0m");
            println!("\x1b[37m{}\x1b[0m", parsed.raw);
            println!();
        }
    }
}
