use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use tauri::{AppHandle, Emitter};

use super::matcher::{mark_episode_watched, mark_movie_watched, match_episode, match_movie};
use super::models::PlexPayload;
use crate::notifications;

/// Shared state for the webhook handler
#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<Sqlite>,
    pub app_handle: AppHandle,
}

/// Event payload for scrobble notifications
#[derive(Clone, Serialize)]
pub struct ScrobbleEvent {
    pub media_type: String,
    pub entity_id: i64,
}

/// Handle incoming Plex webhook
pub async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let pool = &state.pool;
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let payload = match parse_payload(body, content_type).await {
        Ok(payload) => payload,
        Err(error) => {
            eprintln!("[Plex] Invalid webhook: {error}");
            return StatusCode::BAD_REQUEST;
        }
    };

    // Only process media.scrobble events
    if payload.event != "media.scrobble" {
        return StatusCode::OK;
    }

    let metadata = match &payload.metadata {
        Some(m) => m,
        None => {
            return StatusCode::OK;
        }
    };

    match metadata.media_type.as_str() {
        "episode" => {
            let show_name = match &metadata.grandparent_title {
                Some(s) => s.clone(),
                None => {
                    log_scrobble(pool, &payload, None).await;
                    return StatusCode::OK;
                }
            };
            let season = metadata.parent_index.unwrap_or(0);
            let episode = metadata.index.unwrap_or(0);

            let match_result = match_episode(pool, &show_name, season, episode).await;

            if let Some(ref result) = match_result {
                if let Err(e) = mark_episode_watched(pool, result.entity_id).await {
                    eprintln!("[Plex] Failed to mark episode watched: {}", e);
                } else {
                    // Emit event to notify frontend
                    let _ = state.app_handle.emit(
                        "plex-scrobble",
                        ScrobbleEvent {
                            media_type: "episode".to_string(),
                            entity_id: result.entity_id,
                        },
                    );

                    // Create in-app notification
                    let title = format!("Plex: Marked episode as watched");
                    let body = format!("{} S{:02}E{:02}", show_name, season, episode);
                    emit_plex_notification(
                        &state.app_handle,
                        &title,
                        &body,
                        result.entity_id,
                        "episode",
                    )
                    .await;
                }
            }

            log_scrobble(
                pool,
                &payload,
                match_result
                    .as_ref()
                    .map(|r| (r.entity_type.clone(), r.entity_id, r.method.clone())),
            )
            .await;
        }
        "movie" => {
            let match_result = match_movie(
                pool,
                &metadata.title,
                metadata.year,
                metadata
                    .guids
                    .iter()
                    .find_map(|g| g.id.strip_prefix("tmdb://").and_then(|id| id.parse().ok())),
            )
            .await;

            if let Some(ref result) = match_result {
                if let Err(e) = mark_movie_watched(pool, result.entity_id).await {
                    eprintln!("[Plex] Failed to mark movie watched: {}", e);
                } else {
                    // Emit event to notify frontend
                    let _ = state.app_handle.emit(
                        "plex-scrobble",
                        ScrobbleEvent {
                            media_type: "movie".to_string(),
                            entity_id: result.entity_id,
                        },
                    );

                    // Create in-app notification
                    let title = format!("Plex: Marked movie as watched");
                    let body = metadata.title.clone();
                    emit_plex_notification(
                        &state.app_handle,
                        &title,
                        &body,
                        result.entity_id,
                        "movie",
                    )
                    .await;
                }
            }

            log_scrobble(
                pool,
                &payload,
                match_result
                    .as_ref()
                    .map(|r| (r.entity_type.clone(), r.entity_id, r.method.clone())),
            )
            .await;
        }
        _ => {
            // Ignore unknown media types
        }
    }

    StatusCode::OK
}

/// Parse structured multipart bytes; binary thumbnail fields never pass through UTF-8 conversion.
async fn parse_payload(body: Bytes, content_type: &str) -> Result<PlexPayload, String> {
    if !content_type.starts_with("multipart/") {
        return serde_json::from_slice(&body).map_err(|e| e.to_string());
    }
    let boundary = multer::parse_boundary(content_type).map_err(|e| e.to_string())?;
    let stream = axum::body::Body::from(body).into_data_stream();
    let mut multipart = multer::Multipart::new(stream, boundary);
    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        if field.name() == Some("payload") {
            let bytes = field.bytes().await.map_err(|e| e.to_string())?;
            return serde_json::from_slice(&bytes).map_err(|e| e.to_string());
        }
    }
    Err("Missing payload field".into())
}

/// Create an in-app notification for a Plex scrobble event
async fn emit_plex_notification(
    app_handle: &AppHandle,
    title: &str,
    body: &str,
    entity_id: i64,
    reference_type: &str,
) {
    let _ = notifications::send_notification(
        app_handle,
        &notifications::models::CreateNotification {
            r#type: "plex".to_string(),
            title: title.to_string(),
            body: body.to_string(),
            icon: None,
            reference_id: Some(entity_id.to_string()),
            reference_type: Some(reference_type.to_string()),
            expires_at: None,
        },
    )
    .await;
}

/// Log a scrobble event to the database
async fn log_scrobble(
    pool: &Pool<Sqlite>,
    payload: &PlexPayload,
    match_info: Option<(String, i64, String)>,
) {
    let metadata = match &payload.metadata {
        Some(m) => m,
        None => return,
    };

    let (entity_type, entity_id, method) = match match_info {
        Some((t, id, m)) => (Some(t), Some(id), Some(m)),
        None => (None, None, None),
    };

    let result = sqlx::query(
        r#"
        INSERT INTO plex_scrobble_log
        (event_type, media_type, raw_title, show_name, season_number, episode_number, year,
         matched_entity_type, matched_entity_id, match_method)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&payload.event)
    .bind(&metadata.media_type)
    .bind(&metadata.title)
    .bind(&metadata.grandparent_title)
    .bind(metadata.parent_index)
    .bind(metadata.index)
    .bind(metadata.year)
    .bind(entity_type)
    .bind(entity_id)
    .bind(method)
    .execute(pool)
    .await;

    if let Err(e) = result {
        eprintln!("[Plex] Failed to log scrobble: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn unicode_multipart_with_binary_thumbnail_and_malformed_json_never_panics() {
        let title = format!("{}éØ日本", "a".repeat(199));
        let json =
            serde_json::json!({"event":"media.scrobble","Metadata":{"type":"movie","title":title}})
                .to_string();
        let mut body = format!("--test\r\nContent-Disposition: form-data; name=\"thumb\"\r\n\r\n")
            .into_bytes();
        body.extend_from_slice(&[0xff, 0xfe, 0x00]);
        body.extend_from_slice(format!("\r\n--test\r\nContent-Disposition: form-data; name=\"payload\"\r\n\r\n{json}\r\n--test--\r\n").as_bytes());
        assert_eq!(
            parse_payload(body.into(), "multipart/form-data; boundary=test")
                .await
                .unwrap()
                .metadata
                .unwrap()
                .title,
            title
        );
        for body in [
            json.replace("scrobble", "ééé")
                .trim_end_matches('}')
                .to_string(),
            "é".repeat(201),
        ] {
            assert!(parse_payload(Bytes::from(body), "application/json")
                .await
                .is_err());
        }
        assert!(
            parse_payload(Bytes::from_static(b"invalid"), "multipart/form-data")
                .await
                .is_err()
        );
    }
}
