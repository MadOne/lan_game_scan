use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

// =============================================================================
// Shared state
// =============================================================================

type MatchZyState = Arc<RwLock<String>>;

// =============================================================================
// POST /MatchZyConfig
//
// Receives the complete MatchZy JSON and replaces the currently hosted config.
// =============================================================================

async fn post_matchzy_config(State(state): State<MatchZyState>, body: String) -> StatusCode {
    *state.write().await = body;

    StatusCode::OK
}

// =============================================================================
// GET /MatchZyConfig
//
// Returns the currently hosted MatchZy configuration.
// =============================================================================

async fn get_matchzy_config(State(state): State<MatchZyState>) -> (StatusCode, String) {
    let config = state.read().await.clone();

    if config.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            "No MatchZy configuration published.".to_string(),
        );
    }

    (StatusCode::OK, config)
}

// =============================================================================
// POST /MatchZyLogs
//
// Receives log/event payloads sent by MatchZy.
// =============================================================================

async fn post_matchzy_logs(headers: HeaderMap, body: String) -> StatusCode {
    // Optional: Validate matchzy_remote_log_header_key and value if configured
    // if let Some(val) = headers.get("your-header-key") {
    //     // check value...
    // }

    println!("[MATCHZY LOG] Received event payload:\n{}", body);

    StatusCode::OK
}

// =============================================================================
// Server
// =============================================================================

pub async fn start_matchzy_server(addr: SocketAddr) {
    let state: MatchZyState = Arc::new(RwLock::new(String::new()));

    let app = Router::new()
        .route(
            "/MatchZyConfig",
            get(get_matchzy_config).post(post_matchzy_config),
        )
        .route("/MatchZyLogs", post(post_matchzy_logs))
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,

        Err(error) => {
            eprintln!("[MATCHZY HTTP] Failed to bind {}: {}", addr, error);
            return;
        }
    };

    println!("[MATCHZY HTTP] Listening on http://{}", addr);

    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("[MATCHZY HTTP] Server stopped: {}", error);
    }
}
