use crate::db::connection;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

const SETTINGS_KEY: &str = "general_preferences";
const RELEASE_COUNTRIES: &[&str] = &[
    "US", "NO", "GB", "SE", "DK", "FI", "DE", "FR", "NL", "ES", "IT", "CA", "AU",
];
static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(true);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneralPreferences {
    pub startup_page: String,
    pub remember_calendar: bool,
    pub close_action: String,
    pub start_minimized: bool,
    pub release_country: String,
    pub last_page: String,
    pub calendar_view: String,
    pub calendar_filter: String,
    pub default_tier_list: String,
}

impl Default for GeneralPreferences {
    fn default() -> Self {
        Self {
            startup_page: "calendar".into(),
            remember_calendar: false,
            close_action: "tray".into(),
            start_minimized: false,
            release_country: "US".into(),
            last_page: "calendar".into(),
            calendar_view: "month".into(),
            calendar_filter: "all".into(),
            default_tier_list: "shows".into(),
        }
    }
}

impl GeneralPreferences {
    fn validate(&self) -> Result<(), String> {
        if !["calendar", "movies", "shows", "last_used"].contains(&self.startup_page.as_str()) {
            return Err("Choose a valid startup page".into());
        }
        if !["tray", "quit"].contains(&self.close_action.as_str()) {
            return Err("Choose whether closing TVC hides it or quits".into());
        }
        if !RELEASE_COUNTRIES.contains(&self.release_country.as_str()) {
            return Err("Choose a supported release country".into());
        }
        if ![
            "calendar",
            "movies",
            "shows",
            "tiers",
            "racing",
            "awards",
            "statistics",
            "data",
            "settings",
        ]
        .contains(&self.last_page.as_str())
        {
            return Err("Invalid remembered page".into());
        }
        if !["month", "week", "agenda"].contains(&self.calendar_view.as_str()) {
            return Err("Invalid remembered calendar view".into());
        }
        if !["all", "shows", "movies"].contains(&self.calendar_filter.as_str()) {
            return Err("Invalid remembered calendar filter".into());
        }
        if !["shows", "movies"].contains(&self.default_tier_list.as_str()) {
            return Err("Choose TV shows or movies as the default tier list".into());
        }
        Ok(())
    }
}

pub async fn preferences_from_pool(pool: &SqlitePool) -> Result<GeneralPreferences, String> {
    let saved: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(SETTINGS_KEY)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("Could not read general settings: {error}"))?;
    let preferences: GeneralPreferences = match saved {
        Some(value) => serde_json::from_str(&value)
            .map_err(|error| format!("Could not read saved general settings: {error}"))?,
        None => GeneralPreferences::default(),
    };
    preferences.validate()?;
    Ok(preferences)
}

async fn save_preferences(
    pool: &SqlitePool,
    preferences: &GeneralPreferences,
) -> Result<(), String> {
    preferences.validate()?;
    let value = serde_json::to_string(preferences)
        .map_err(|error| format!("Could not prepare general settings: {error}"))?;
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(SETTINGS_KEY)
        .bind(value)
        .execute(pool)
        .await
        .map_err(|error| format!("Could not save general settings: {error}"))?;
    CLOSE_TO_TRAY.store(preferences.close_action == "tray", Ordering::Relaxed);
    Ok(())
}

pub fn close_to_tray() -> bool {
    CLOSE_TO_TRAY.load(Ordering::Relaxed)
}

pub async fn release_country(pool: &SqlitePool) -> Result<String, String> {
    Ok(preferences_from_pool(pool).await?.release_country)
}

#[tauri::command]
pub async fn get_general_preferences(app: AppHandle) -> Result<GeneralPreferences, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|error| format!("Database error: {error}"))?;
    let preferences = preferences_from_pool(&pool).await?;
    CLOSE_TO_TRAY.store(preferences.close_action == "tray", Ordering::Relaxed);
    Ok(preferences)
}

#[tauri::command]
pub async fn update_general_preferences(
    app: AppHandle,
    preferences: GeneralPreferences,
) -> Result<GeneralPreferences, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|error| format!("Database error: {error}"))?;
    save_preferences(&pool, &preferences).await?;
    Ok(preferences)
}

#[tauri::command]
pub async fn apply_startup_window_preferences(app: AppHandle) -> Result<(), String> {
    // A normal launch stays visible, including manual development testing.
    if !std::env::args().any(|argument| argument == "--autostart") {
        return Ok(());
    }
    let preferences = get_general_preferences(app.clone()).await?;
    if preferences.start_minimized {
        if let Some(window) = app.get_webview_window("main") {
            window
                .hide()
                .map_err(|error| format!("Could not start TVC in the tray: {error}"))?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct WindowsStartup {
    pub enabled: bool,
    pub supported: bool,
}

fn windows_startup_supported() -> bool {
    cfg!(target_os = "windows") && !cfg!(debug_assertions)
}

fn startup_command(executable: &std::path::Path) -> Result<String, String> {
    let path = executable
        .to_str()
        .ok_or("The TVC executable path cannot be used for Windows startup")?;
    if path.contains('"') {
        return Err("The TVC executable path cannot contain quotation marks".into());
    }
    Ok(format!("\"{path}\" --autostart"))
}

#[cfg(target_os = "windows")]
fn read_windows_startup() -> Result<bool, String> {
    use std::io::ErrorKind;
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let key = match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
    {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("Could not read Windows startup: {error}")),
    };
    let saved: String = match key.get_value(crate::db::app_identifier()) {
        Ok(value) => value,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "Could not read TVC's Windows startup setting: {error}"
            ))
        }
    };
    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not locate TVC for Windows startup: {error}"))?;
    Ok(saved == startup_command(&executable)?)
}

#[tauri::command]
pub fn get_start_with_windows() -> Result<WindowsStartup, String> {
    if !windows_startup_supported() {
        // Debug builds never read or write the installed release's startup entry.
        return Ok(WindowsStartup {
            enabled: false,
            supported: false,
        });
    }
    #[cfg(target_os = "windows")]
    return Ok(WindowsStartup {
        enabled: read_windows_startup()?,
        supported: true,
    });
    #[cfg(not(target_os = "windows"))]
    Ok(WindowsStartup {
        enabled: false,
        supported: false,
    })
}

#[tauri::command]
pub fn set_start_with_windows(enabled: bool) -> Result<WindowsStartup, String> {
    if !windows_startup_supported() {
        if enabled {
            return Err("Start with Windows is available in installed releases".into());
        }
        return get_start_with_windows();
    }
    #[cfg(target_os = "windows")]
    {
        use std::io::ErrorKind;
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
            .map_err(|error| format!("Could not open Windows startup: {error}"))?;
        // This build owns exactly one value. No other startup entries are changed.
        if enabled {
            let executable = std::env::current_exe()
                .map_err(|error| format!("Could not locate TVC for Windows startup: {error}"))?;
            key.set_value(crate::db::app_identifier(), &startup_command(&executable)?)
                .map_err(|error| format!("Could not enable TVC at Windows startup: {error}"))?;
        } else if let Err(error) = key.delete_value(crate::db::app_identifier()) {
            if error.kind() != ErrorKind::NotFound {
                return Err(format!("Could not disable TVC at Windows startup: {error}"));
            }
        }
    }
    get_start_with_windows()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn preferences_default_and_roundtrip_without_touching_library_data() {
        let pool = crate::audit_tests::database().await;
        let defaults = preferences_from_pool(&pool).await.unwrap();
        assert_eq!(defaults.release_country, "US");
        assert_eq!(defaults.close_action, "tray");
        assert_eq!(defaults.default_tier_list, "shows");
        let preferences = GeneralPreferences {
            startup_page: "last_used".into(),
            remember_calendar: true,
            close_action: "quit".into(),
            start_minimized: true,
            release_country: "NO".into(),
            last_page: "movies".into(),
            calendar_view: "week".into(),
            calendar_filter: "movies".into(),
            default_tier_list: "movies".into(),
        };
        save_preferences(&pool, &preferences).await.unwrap();
        assert_eq!(preferences_from_pool(&pool).await.unwrap(), preferences);
        assert_eq!(release_country(&pool).await.unwrap(), "NO");
        let mut invalid = preferences.clone();
        invalid.release_country = "ZZ".into();
        assert!(save_preferences(&pool, &invalid).await.is_err());
        assert_eq!(preferences_from_pool(&pool).await.unwrap(), preferences);
    }

    #[test]
    fn invalid_navigation_and_calendar_preferences_are_rejected() {
        for field in [
            "startup",
            "close",
            "page",
            "view",
            "filter",
            "country",
            "tier_list",
        ] {
            let mut preferences = GeneralPreferences::default();
            match field {
                "startup" => preferences.startup_page = "invalid".into(),
                "close" => preferences.close_action = "invalid".into(),
                "page" => preferences.last_page = "invalid".into(),
                "view" => preferences.calendar_view = "invalid".into(),
                "filter" => preferences.calendar_filter = "invalid".into(),
                "tier_list" => preferences.default_tier_list = "invalid".into(),
                _ => preferences.release_country = "us".into(),
            }
            assert!(preferences.validate().is_err(), "{field}");
        }
    }

    #[tokio::test]
    async fn older_preferences_default_to_shows_without_replacing_other_choices() {
        let pool = crate::audit_tests::database().await;
        let mut saved = serde_json::to_value(GeneralPreferences::default()).unwrap();
        saved.as_object_mut().unwrap().remove("default_tier_list");
        saved["release_country"] = serde_json::json!("NO");
        sqlx::query("INSERT INTO settings(key,value) VALUES (?,?)")
            .bind(SETTINGS_KEY)
            .bind(saved.to_string())
            .execute(&pool)
            .await
            .unwrap();
        let preferences = preferences_from_pool(&pool).await.unwrap();
        assert_eq!(preferences.default_tier_list, "shows");
        assert_eq!(preferences.release_country, "NO");
    }

    #[test]
    fn startup_command_quotes_the_executable_and_debug_never_touches_registry() {
        let path = std::path::Path::new("C:\\Program Files\\TVC\\tvc.exe");
        assert_eq!(
            startup_command(path).unwrap(),
            "\"C:\\Program Files\\TVC\\tvc.exe\" --autostart"
        );
        if cfg!(debug_assertions) {
            let status = get_start_with_windows().unwrap();
            assert!(!status.enabled && !status.supported);
            assert!(set_start_with_windows(true).is_err());
            assert!(!set_start_with_windows(false).unwrap().enabled);
            assert_eq!(crate::db::app_identifier(), "com.tvc.app.dev");
        }
    }
}
