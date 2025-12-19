use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;
use crate::db::connection;
use crate::tmdb;
use crate::tvdb;

#[derive(Debug, Serialize, Deserialize)]
pub struct CastMemberData {
    pub id: i64,
    pub person_id: Option<i64>,
    pub name: String,
    pub character_name: Option<String>,
    pub image_url: Option<String>,
    pub order_index: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrewMemberData {
    pub id: i64,
    pub person_id: Option<i64>,
    pub name: String,
    pub job: Option<String>,
    pub department: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CastCrewData {
    pub cast: Vec<CastMemberData>,
    pub crew: Vec<CrewMemberData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrailerData {
    pub name: String,
    pub url: String,
    pub embed_url: Option<String>,
    pub site: String,
}

/// Fetch and store cast/crew for a movie from TMDB
#[tauri::command]
pub async fn fetch_movie_cast_crew(
    app: AppHandle,
    movie_id: i64,
) -> Result<CastCrewData, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // movie_id IS the TMDB ID (primary key in movies table)
    // Fetch credits from TMDB
    let credits = tmdb::get_movie_credits(movie_id)
        .await
        .map_err(|e| format!("Failed to fetch credits: {}", e))?;

    // Clear existing cast/crew for this movie
    sqlx::query(r#"DELETE FROM cast_members WHERE movie_id = ?"#)
        .bind(movie_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to clear cast: {}", e))?;

    sqlx::query(r#"DELETE FROM crew_members WHERE movie_id = ?"#)
        .bind(movie_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to clear crew: {}", e))?;

    // Insert cast (limit to top 20)
    let cast_data: Vec<CastMemberData> = credits.cast.iter().take(20).enumerate().map(|(idx, c)| {
        CastMemberData {
            id: 0, // Will be set by DB
            person_id: Some(c.id),
            name: c.name.clone(),
            character_name: c.character.clone(),
            image_url: c.image_url(),
            order_index: c.order.unwrap_or(idx as i32),
        }
    }).collect();

    for cast in &cast_data {
        sqlx::query(
            r#"INSERT INTO cast_members (movie_id, person_id, name, character_name, order_index, image_url)
               VALUES (?, ?, ?, ?, ?, ?)"#
        )
        .bind(movie_id)
        .bind(cast.person_id)
        .bind(&cast.name)
        .bind(&cast.character_name)
        .bind(cast.order_index)
        .bind(&cast.image_url)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to insert cast: {}", e))?;
    }

    // Insert key crew (directors, writers, producers - limit to 10)
    let important_jobs = ["Director", "Writer", "Screenplay", "Producer", "Executive Producer", "Composer"];
    let crew_data: Vec<CrewMemberData> = credits.crew.iter()
        .filter(|c| c.job.as_ref().map(|j| important_jobs.iter().any(|ij| j.contains(ij))).unwrap_or(false))
        .take(10)
        .map(|c| {
            CrewMemberData {
                id: 0,
                person_id: Some(c.id),
                name: c.name.clone(),
                job: c.job.clone(),
                department: c.department.clone(),
                image_url: c.image_url(),
            }
        })
        .collect();

    for crew in &crew_data {
        sqlx::query(
            r#"INSERT INTO crew_members (movie_id, person_id, name, job, department, image_url)
               VALUES (?, ?, ?, ?, ?, ?)"#
        )
        .bind(movie_id)
        .bind(crew.person_id)
        .bind(&crew.name)
        .bind(&crew.job)
        .bind(&crew.department)
        .bind(&crew.image_url)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to insert crew: {}", e))?;
    }

    Ok(CastCrewData {
        cast: cast_data,
        crew: crew_data,
    })
}

/// Fetch and store cast for a show from TVDB
#[tauri::command]
pub async fn fetch_show_cast(
    app: AppHandle,
    show_id: i64,
) -> Result<Vec<CastMemberData>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Fetch characters from TVDB (show_id IS the TVDB ID)
    let characters = tvdb::get_series_characters(show_id)
        .await
        .map_err(|e| format!("Failed to fetch characters: {}", e))?;

    // Clear existing cast for this show
    sqlx::query(r#"DELETE FROM cast_members WHERE show_id = ?"#)
        .bind(show_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to clear cast: {}", e))?;

    // Insert cast (limit to top 20)
    let cast_data: Vec<CastMemberData> = characters.iter().take(20).enumerate().map(|(idx, c)| {
        CastMemberData {
            id: 0,
            person_id: c.people_id,
            name: c.person_name.clone().unwrap_or_else(|| "Unknown".to_string()),
            character_name: c.name.clone(),
            image_url: c.image.clone(),
            order_index: c.sort.unwrap_or(idx as i32),
        }
    }).collect();

    for cast in &cast_data {
        sqlx::query(
            r#"INSERT INTO cast_members (show_id, person_id, name, character_name, order_index, image_url)
               VALUES (?, ?, ?, ?, ?, ?)"#
        )
        .bind(show_id)
        .bind(cast.person_id)
        .bind(&cast.name)
        .bind(&cast.character_name)
        .bind(cast.order_index)
        .bind(&cast.image_url)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to insert cast: {}", e))?;
    }

    Ok(cast_data)
}

/// Get stored cast/crew for a movie
#[tauri::command]
pub async fn get_movie_cast_crew(
    app: AppHandle,
    movie_id: i64,
) -> Result<CastCrewData, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let cast_rows = sqlx::query(
        r#"SELECT id, person_id, name, character_name, order_index, image_url
           FROM cast_members WHERE movie_id = ? ORDER BY order_index"#
    )
    .bind(movie_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to fetch cast: {}", e))?;

    let cast: Vec<CastMemberData> = cast_rows.iter().map(|row| {
        CastMemberData {
            id: row.get("id"),
            person_id: row.get("person_id"),
            name: row.get("name"),
            character_name: row.get("character_name"),
            image_url: row.get("image_url"),
            order_index: row.get("order_index"),
        }
    }).collect();

    let crew_rows = sqlx::query(
        r#"SELECT id, person_id, name, job, department, image_url
           FROM crew_members WHERE movie_id = ?"#
    )
    .bind(movie_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to fetch crew: {}", e))?;

    let crew: Vec<CrewMemberData> = crew_rows.iter().map(|row| {
        CrewMemberData {
            id: row.get("id"),
            person_id: row.get("person_id"),
            name: row.get("name"),
            job: row.get("job"),
            department: row.get("department"),
            image_url: row.get("image_url"),
        }
    }).collect();

    Ok(CastCrewData { cast, crew })
}

/// Get stored cast for a show
#[tauri::command]
pub async fn get_show_cast(
    app: AppHandle,
    show_id: i64,
) -> Result<Vec<CastMemberData>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let cast_rows = sqlx::query(
        r#"SELECT id, person_id, name, character_name, order_index, image_url
           FROM cast_members WHERE show_id = ? ORDER BY order_index"#
    )
    .bind(show_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to fetch cast: {}", e))?;

    let cast: Vec<CastMemberData> = cast_rows.iter().map(|row| {
        CastMemberData {
            id: row.get("id"),
            person_id: row.get("person_id"),
            name: row.get("name"),
            character_name: row.get("character_name"),
            image_url: row.get("image_url"),
            order_index: row.get("order_index"),
        }
    }).collect();

    Ok(cast)
}

/// Get movie trailer from TMDB
#[tauri::command]
pub async fn get_movie_trailer(
    _app: AppHandle,
    movie_id: i64,
) -> Result<Option<TrailerData>, String> {
    // movie_id IS the TMDB ID (primary key in movies table)
    let trailer = tmdb::get_movie_trailer(movie_id)
        .await
        .map_err(|e| format!("Failed to fetch trailer: {}", e))?;

    Ok(trailer.map(|t| {
        let url = t.youtube_url().unwrap_or_default();
        let embed_url = t.youtube_embed_url();
        TrailerData {
            name: t.name,
            url,
            embed_url,
            site: t.site,
        }
    }))
}

/// Get show trailer from TMDB (searches by show name since we use TVDB IDs)
#[tauri::command]
pub async fn get_show_trailer(
    app: AppHandle,
    show_id: i64,
) -> Result<Option<TrailerData>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Get the show name from database
    let show_name: String = sqlx::query(r#"SELECT name FROM shows WHERE id = ?"#)
        .bind(show_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| format!("Show not found: {}", e))?
        .get("name");

    // Search TMDB for the show to get TMDB ID
    let search_results = tmdb::search_tv_show(&show_name)
        .await
        .map_err(|e| format!("Failed to search TMDB: {}", e))?;

    // Get the first result (best match)
    let tmdb_show = search_results.first()
        .ok_or_else(|| "Show not found on TMDB".to_string())?;

    // Fetch trailer from TMDB
    let trailer = tmdb::get_tv_trailer(tmdb_show.id)
        .await
        .map_err(|e| format!("Failed to fetch trailer: {}", e))?;

    Ok(trailer.map(|t| {
        let url = t.youtube_url().unwrap_or_default();
        let embed_url = t.youtube_embed_url();
        TrailerData {
            name: t.name,
            url,
            embed_url,
            site: t.site,
        }
    }))
}
