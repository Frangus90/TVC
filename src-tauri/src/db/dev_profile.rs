//! Move development into its own identity without losing the existing dev library.
//! The only legacy filename this code can open is tvc_dev.db.
use std::path::Path;

pub fn prepare() -> Result<(), String> {
    let base = dirs::data_dir().ok_or("Cannot find application data directory")?;
    copy_legacy_dev(&base)
}

fn copy_legacy_dev(base: &Path) -> Result<(), String> {
    let source = base.join("com.tvc.app").join("tvc_dev.db");
    let destination = base.join("com.tvc.app.dev").join("tvc_dev.db");
    if destination.exists() || !source.exists() {
        return Ok(());
    }
    let parent = destination
        .parent()
        .ok_or("Missing dev profile directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!("tvc_dev.{}.copy", std::process::id()));
    // SQLite copies a consistent snapshot, including committed WAL contents.
    // A raw file copy could omit recently saved development data.
    let conn =
        rusqlite::Connection::open_with_flags(&source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("Cannot read previous development database: {e}"))?;
    conn.execute("VACUUM main INTO ?", [temporary.to_string_lossy().as_ref()])
        .map_err(|e| format!("Cannot copy development profile: {e}"))?;
    drop(conn);
    std::fs::rename(&temporary, &destination)
        .map_err(|e| format!("Cannot install development profile: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn copies_only_legacy_dev_and_never_overwrites_an_existing_profile() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "tvc-dev-profile-test-{}-{unique}",
            std::process::id()
        ));
        let old = base.join("com.tvc.app");
        std::fs::create_dir_all(&old).unwrap();
        let source = old.join("tvc_dev.db");
        let conn = rusqlite::Connection::open(&source).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.execute("CREATE TABLE fixture (value TEXT)", [])
            .unwrap();
        conn.execute("INSERT INTO fixture VALUES ('original')", [])
            .unwrap();
        super::copy_legacy_dev(&base).unwrap();
        let dest = base.join("com.tvc.app.dev").join("tvc_dev.db");
        let copy = rusqlite::Connection::open(&dest).unwrap();
        let value: String = copy
            .query_row("SELECT value FROM fixture", [], |r| r.get(0))
            .unwrap();
        assert_eq!(value, "original");
        conn.execute("UPDATE fixture SET value = 'later'", [])
            .unwrap();
        super::copy_legacy_dev(&base).unwrap();
        let value: String = copy
            .query_row("SELECT value FROM fixture", [], |r| r.get(0))
            .unwrap();
        assert_eq!(value, "original");
        drop(copy);
        drop(conn);
        // Remove only the exact disposable files/directories created by this test.
        std::fs::remove_file(dest).unwrap();
        std::fs::remove_file(source).unwrap();
        std::fs::remove_dir(base.join("com.tvc.app.dev")).unwrap();
        std::fs::remove_dir(old).unwrap();
        std::fs::remove_dir(base).unwrap();
    }
}
