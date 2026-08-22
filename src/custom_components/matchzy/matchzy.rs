use axum::{extract::State, http::StatusCode, routing::get, Router};
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
// Server
// =============================================================================

pub async fn start_matchzy_server(addr: SocketAddr) {
    let state: MatchZyState = Arc::new(RwLock::new(String::new()));

    let app = Router::new()
        .route(
            "/MatchZyConfig",
            get(get_matchzy_config).post(post_matchzy_config),
        )
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
