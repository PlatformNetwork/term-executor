use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::handlers::AppState;
use crate::session::WsEvent;

#[derive(Deserialize)]
pub struct WsQuery {
    pub batch_id: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> Response {
    let batch_id = query.batch_id;
    ws.on_upgrade(move |socket| handle_ws(socket, state, batch_id))
}

async fn handle_ws(socket: WebSocket, state: Arc<AppState>, batch_id: String) {
    let batch = match state.sessions.get(&batch_id) {
        Some(b) => b,
        None => {
            let (mut sender, _) = socket.split();
            let err = serde_json::json!({
                "error": "batch_not_found",
                "batch_id": batch_id,
            });
            let msg = serde_json::to_string(&err).unwrap_or_default();
            let _ = sender.send(Message::Text(msg)).await;
            return;
        }
    };

    info!("WebSocket connected for batch {}", batch_id);

    let mut rx: broadcast::Receiver<WsEvent> = batch.events_tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    let current_state = batch.result.lock().await;
    let snapshot = serde_json::json!({
        "event": "snapshot",
        "batch_id": batch_id,
        "data": {
            "status": current_state.status,
            "total_tasks": current_state.total_tasks,
            "completed_tasks": current_state.completed_tasks,
            "passed_tasks": current_state.passed_tasks,
            "failed_tasks": current_state.failed_tasks,
            "aggregate_reward": current_state.aggregate_reward,
            "tasks": current_state.tasks,
        }
    });
    drop(current_state);

    let snapshot_json = serde_json::to_string(&snapshot).unwrap_or_default();
    if sender.send(Message::Text(snapshot_json)).await.is_err() {
        return;
    }

    let batch_id_send = batch_id.clone();
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(_) => continue,
                    };
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    debug!("WebSocket lagged by {} messages", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    let close_msg = serde_json::json!({
                        "event": "stream_closed",
                        "batch_id": batch_id_send,
                    });
                    let close_json = serde_json::to_string(&close_msg).unwrap_or_default();
                    let _ = sender.send(Message::Text(close_json)).await;
                    break;
                }
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(data)) => {
                    debug!("Received ping");
                    let _ = data;
                }
                Err(e) => {
                    warn!("WebSocket receive error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    info!("WebSocket disconnected for batch {}", batch_id);
}
