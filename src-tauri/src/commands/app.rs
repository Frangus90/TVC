use tauri::AppHandle;

/// Fully exit the application (not just minimize to tray)
#[tauri::command]
pub async fn exit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
