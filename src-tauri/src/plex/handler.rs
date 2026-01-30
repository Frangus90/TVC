use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
};
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use tauri::{AppHandle, Emitter};

use super::matcher::{match_episode, match_movie, mark_episode_watched, mark_movie_watched};
use super::models::PlexPayload;

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
    body: Bytes,
) -> StatusCode {
    let pool = &state.pool;
    // Log the raw body for debugging (first 200 chars)
    let _body_preview = String::from_utf8_lossy(&body);
    let _preview = if _body_preview.len() > 200 {
        &_body_preview[..200]
    } else {
        &_body_preview
    };
    // Removed debug println! statements - use proper logging in production
    // Only log errors with eprintln! for now
    
    // TODO: Add webhook authentication/authorization
    // Security Note: Currently webhook is only accessible from localhost (127.0.0.1)
    // For production hardening, consider:
    // 1. Verify Plex webhook signature if available
    // 2. Require a configurable token in request header
    // 3. Use IP whitelist (already limited to 127.0.0.1)

    // Parse multipart form data (Plex sends the JSON in a multipart payload field)
    let payload = match parse_multipart_payload(&body) {
        Some(p) => p,
        None => {
            // Try parsing as raw JSON (some Plex versions or test tools)
            match serde_json::from_slice::<PlexPayload>(&body) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[Plex] Failed to parse webhook payload: {}", e);
                    // Return OK anyway to not cause Plex to retry
                    return StatusCode::OK;
                }
            }
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
                    let _ = state.app_handle.emit("plex-scrobble", ScrobbleEvent {
                        media_type: "episode".to_string(),
                        entity_id: result.entity_id,
                    });
                }
            }

            log_scrobble(pool, &payload, match_result.as_ref().map(|r| (r.entity_type.clone(), r.entity_id, r.method.clone()))).await;
        }
        "movie" => {
            let match_result = match_movie(pool, &metadata.title, metadata.year).await;

            if let Some(ref result) = match_result {
                if let Err(e) = mark_movie_watched(pool, result.entity_id).await {
                    eprintln!("[Plex] Failed to mark movie watched: {}", e);
                } else {
                    // Emit event to notify frontend
                    let _ = state.app_handle.emit("plex-scrobble", ScrobbleEvent {
                        media_type: "movie".to_string(),
                        entity_id: result.entity_id,
                    });
                }
            }

            log_scrobble(pool, &payload, match_result.as_ref().map(|r| (r.entity_type.clone(), r.entity_id, r.method.clone()))).await;
        }
        _ => {
            // Ignore unknown media types
        }
    }

    StatusCode::OK
}

/// Parse multipart form data to extract the JSON payload
fn parse_multipart_payload(body: &Bytes) -> Option<PlexPayload> {
    let body_str = String::from_utf8_lossy(body);

    // Plex webhook format: multipart/form-data with "payload" JSON field
    // May also contain binary "thumb" field which can corrupt brace-matching
    //
    // Format:
    // --boundary\r\n
    // Content-Disposition: form-data; name="payload"\r\n
    // Content-Type: application/json\r\n
    // \r\n
    // {JSON}\r\n
    // --boundary\r\n
    // ...more parts...
    // --boundary--

    // Find the payload field
    let payload_start = body_str.find("name=\"payload\"")
        .or_else(|| body_str.find("name='payload'"))?;

    // Find the empty line after headers (marks start of content)
    let after_name = &body_str[payload_start..];
    let content_start = after_name.find("\r\n\r\n")
        .map(|p| p + 4)
        .or_else(|| after_name.find("\n\n").map(|p| p + 2))?;

    let json_start_abs = payload_start + content_start;
    let json_content = &body_str[json_start_abs..];

    // Find where JSON ends: look for the next multipart boundary
    // Boundaries start with \r\n-- or \n--
    let json_end = json_content.find("\r\n--")
        .or_else(|| json_content.find("\n--"))
        .unwrap_or(json_content.len());

    let json_str = json_content[..json_end].trim();

    match serde_json::from_str::<PlexPayload>(json_str) {
        Ok(payload) => Some(payload),
        Err(e) => {
            eprintln!("[Plex] JSON parse failed: {} at line {} col {}", e, e.line(), e.column());
            // Show context around error for debugging
            let col = e.column().saturating_sub(1);
            if col < json_str.len() {
                let start = col.saturating_sub(50);
                let end = (col + 50).min(json_str.len());
                eprintln!("[Plex] Context around error: ...{}...", &json_str[start..end]);
            }
            None
        }
    }
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
        "#
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
