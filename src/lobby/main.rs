mod handlers;
mod k8s;
mod room_code;
mod room_store;

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tokio::net::TcpListener;

use room_store::RoomStore;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let room_story = Arc::new(RoomStore::new());

    let k8s_client = k8s::create_client().await?;

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/rooms/create", post(handlers::create_room))
        .route("/rooms/register", post(handlers::register_room))
        .route("/rooms/{code}", get(handlers::get_room))
        .with_state((room_story, k8s_client));

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("Lobby server listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
