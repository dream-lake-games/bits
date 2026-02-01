use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{k8s, room_code, room_store::RoomStore};

type AppState = (Arc<RoomStore>, k8s::K8sClient);

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

#[derive(Serialize)]
pub struct CreateRoomResponse {
    room_code: String,
}

pub async fn create_room(
    State((room_store, k8s_client)): State<AppState>,
) -> Result<Json<CreateRoomResponse>, StatusCode> {
    let room_code = room_code::generate();
    let response_room_code = room_code.clone();
    let room_info = crate::room_store::RoomInfo::new(room_code.clone());

    room_store.insert(room_code.clone(), room_info);

    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".to_string());
    let lobby_url =
        std::env::var("LOBBY_URL").unwrap_or_else(|_| "http://lobby:8080".to_string());

    if let Err(e) = k8s::create_game_server_job(&k8s_client, &namespace, &room_code, &lobby_url)
        .await
    {
        tracing::error!("Failed to create job: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    if let Err(e) = k8s::create_loadbalancer_service(&k8s_client, &namespace, &room_code).await {
        tracing::error!("Failed to create service: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let room_code_clone = room_code.clone();
    let room_store_clone = room_store.clone();
    let k8s_client_clone = k8s_client.clone();
    let namespace_clone = namespace.clone();
    tokio::spawn(async move {
        let mut attempts = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            attempts += 1;

            if attempts > 30 {
                tracing::warn!("Gave up waiting for LoadBalancer address for {}", room_code_clone);
                break;
            }

            match k8s::get_service_address(&k8s_client_clone, &namespace_clone, &room_code_clone).await {
                Ok(Some(address)) => {
                    tracing::info!("LoadBalancer ready for {}: {}", room_code_clone, address);
                    room_store_clone.update_server_address(&room_code_clone, address);
                    break;
                }
                Ok(None) => {
                    tracing::debug!("LoadBalancer not ready yet for {}", room_code_clone);
                }
                Err(e) => {
                    tracing::error!("Error checking service: {}", e);
                    break;
                }
            }
        }
    });

    Ok(Json(CreateRoomResponse { room_code: response_room_code }))
}

#[derive(Deserialize)]
pub struct RegisterRoomRequest {
    room_code: String,
    cert_hash: String,
}

pub async fn register_room(
    State((room_store, _)): State<AppState>,
    Json(payload): Json<RegisterRoomRequest>,
) -> StatusCode {
    if room_store.update_cert_hash(&payload.room_code, payload.cert_hash.clone()) {
        tracing::info!("Registered room {} with cert hash", payload.room_code);
        StatusCode::OK
    } else {
        tracing::warn!("Room {} not found for registration", payload.room_code);
        StatusCode::NOT_FOUND
    }
}

#[derive(Serialize)]
#[serde(tag = "status")]
pub enum GetRoomResponse {
    #[serde(rename = "pending")]
    Pending { room_code: String },
    #[serde(rename = "ready")]
    Ready {
        room_code: String,
        server_address: String,
        cert_hash: String,
    },
}

pub async fn get_room(
    State((room_store, _)): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<GetRoomResponse>, StatusCode> {
    let room_info = room_store.get(&code).ok_or(StatusCode::NOT_FOUND)?;

    if room_info.is_ready() {
        Ok(Json(GetRoomResponse::Ready {
            room_code: room_info.room_code,
            server_address: room_info.server_address.unwrap(),
            cert_hash: room_info.cert_hash.unwrap(),
        }))
    } else {
        Ok(Json(GetRoomResponse::Pending {
            room_code: room_info.room_code,
        }))
    }
}

