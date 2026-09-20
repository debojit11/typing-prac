use crate::auth::AccountsDb;
use std::{fs, io::Write, path::PathBuf};
use tauri::Manager;

fn path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("accounts.json"))
        .map_err(|e| e.to_string())
}

pub fn load(app: &tauri::AppHandle) -> Result<AccountsDb, String> {
    let path = path(app)?;
    let backup = path.with_extension("bak");
    if !path.exists() && !backup.exists() {
        return Ok(AccountsDb::default());
    }
    for candidate in [&path, &backup] {
        if let Ok(text) = fs::read_to_string(candidate) {
            if let Ok(db) = serde_json::from_str(&text) {
                return Ok(db);
            }
        }
    }
    Err("Local account data is damaged".into())
}

pub fn save(app: &tauri::AppHandle, db: &AccountsDb) -> Result<(), String> {
    let path = path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec(db).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    let backup = path.with_extension("bak");
    let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    if path.exists() {
        fs::copy(&path, &backup).map_err(|e| e.to_string())?;
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    if let Err(error) = fs::rename(&temporary, &path) {
        if backup.exists() {
            let _ = fs::copy(&backup, &path);
        }
        return Err(error.to_string());
    }
    Ok(())
}
