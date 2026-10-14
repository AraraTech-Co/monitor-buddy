use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use shared::{AgentToServer, ServerToAgent};

use crate::{auth::hash_token, models::Host, state::AppState};

pub async fn agent_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<ServerToAgent>();

    // Writer task
    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(text) => {
                    if sink.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("serialize server message: {e}");
                    break;
                }
            }
        }
    });

    let mut host_id: Option<String> = None;
    let mut authenticated = false;

    while let Some(Ok(msg)) = stream.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => break,
            Message::Binary(_) => continue,
        };

        let parsed: AgentToServer = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("invalid agent message: {e}");
                let _ = out_tx.send(ServerToAgent::AuthFail {
                    reason: "invalid json".into(),
                });
                break;
            }
        };

        if !authenticated {
            match parsed {
                AgentToServer::Auth { token } => {
                    let token_hash = hash_token(&token);
                    let host = sqlx::query_as::<_, Host>(
                        "SELECT id, name, agent_token_hash, last_seen_at, created_at FROM hosts WHERE agent_token_hash = ?",
                    )
                    .bind(&token_hash)
                    .fetch_optional(&state.db)
                    .await;

                    match host {
                        Ok(Some(host)) => {
                            host_id = Some(host.id.clone());
                            authenticated = true;
                            state.hub.register(host.id.clone(), out_tx.clone()).await;
                            let _ = sqlx::query(
                                "UPDATE hosts SET last_seen_at = ? WHERE id = ?",
                            )
                            .bind(Utc::now())
                            .bind(&host.id)
                            .execute(&state.db)
                            .await;
                            let _ = out_tx.send(ServerToAgent::AuthOk {
                                host_id: host.id,
                                host_name: host.name,
                            });
                        }
                        Ok(None) => {
                            let _ = out_tx.send(ServerToAgent::AuthFail {
                                reason: "invalid token".into(),
                            });
                            break;
                        }
                        Err(e) => {
                            tracing::error!("auth lookup failed: {e}");
                            let _ = out_tx.send(ServerToAgent::AuthFail {
                                reason: "internal error".into(),
                            });
                            break;
                        }
                    }
                }
                _ => {
                    let _ = out_tx.send(ServerToAgent::AuthFail {
                        reason: "authenticate first".into(),
                    });
                    break;
                }
            }
            continue;
        }

        let hid = host_id.as_deref().unwrap();
        if matches!(&parsed, AgentToServer::Heartbeat { .. }) {
            let _ = sqlx::query("UPDATE hosts SET last_seen_at = ? WHERE id = ?")
                .bind(Utc::now())
                .bind(hid)
                .execute(&state.db)
                .await;
        }
        state.hub.dispatch_agent_message(hid, parsed).await;
    }

    if let Some(hid) = host_id {
        state.hub.disconnect(&hid).await;
    }
    writer.abort();
}
