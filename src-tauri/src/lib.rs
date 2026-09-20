mod auth;
mod generator;
mod model;
mod storage;

use auth::{
    account_key, hash_password, validate_credentials, verify_password, Account, AccountStatus,
    AuthState,
};
use generator::generate;
use model::{AppData, GenerateRequest, SessionPlan, SessionRecord};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use storage::{load, save};
use tauri::{Manager, State};
use zeroize::Zeroize;

struct Store(Mutex<AuthState>);

fn active_data(state: &AuthState) -> Result<&AppData, String> {
    let key = state.active.as_ref().ok_or("Sign in required")?;
    state
        .db
        .accounts
        .get(key)
        .map(|a| &a.data)
        .ok_or("Sign in required".into())
}

fn active_data_mut(state: &mut AuthState) -> Result<&mut AppData, String> {
    let key = state.active.as_ref().ok_or("Sign in required")?.clone();
    state
        .db
        .accounts
        .get_mut(&key)
        .map(|a| &mut a.data)
        .ok_or("Sign in required".into())
}

#[tauri::command]
fn account_status(state: State<'_, Store>) -> Result<AccountStatus, String> {
    let state = state.0.lock().map_err(|_| "Account store is unavailable")?;
    Ok(AccountStatus {
        has_accounts: !state.db.accounts.is_empty(),
        active_username: state
            .active
            .as_ref()
            .and_then(|key| state.db.accounts.get(key))
            .map(|a| a.username.clone()),
    })
}

#[tauri::command]
fn create_account(
    mut username: String,
    mut password: String,
    state: State<'_, Store>,
    app: tauri::AppHandle,
) -> Result<AppData, String> {
    let result = (|| {
        username = username.trim().to_string();
        validate_credentials(&username, &password)?;
        let key = account_key(&username);
        let mut state = state.0.lock().map_err(|_| "Account store is unavailable")?;
        if state.db.accounts.contains_key(&key) {
            return Err("That username is already in use".into());
        }
        let password_hash = hash_password(&password)?;
        let data = AppData::default();
        state.db.accounts.insert(
            key.clone(),
            Account {
                username: username.clone(),
                password_hash,
                data: data.clone(),
            },
        );
        if let Err(error) = save(&app, &state.db) {
            state.db.accounts.remove(&key);
            return Err(error);
        }
        state.active = Some(key);
        Ok(data)
    })();
    username.zeroize();
    password.zeroize();
    result
}

#[tauri::command]
fn login(
    mut username: String,
    mut password: String,
    state: State<'_, Store>,
) -> Result<AppData, String> {
    let result = (|| {
        let key = account_key(username.trim());
        let mut state = state.0.lock().map_err(|_| "Account store is unavailable")?;
        if state
            .retry_after
            .is_some_and(|until| Instant::now() < until)
        {
            return Err("Too many attempts. Try again shortly".into());
        }
        let valid = state
            .db
            .accounts
            .get(&key)
            .is_some_and(|account| verify_password(&password, &account.password_hash));
        if !valid {
            state.failed_logins = state.failed_logins.saturating_add(1);
            if state.failed_logins >= 5 {
                state.retry_after = Some(Instant::now() + Duration::from_secs(5));
                state.failed_logins = 0;
            }
            return Err("Invalid username or password".into());
        }
        state.failed_logins = 0;
        state.retry_after = None;
        state.active = Some(key);
        active_data(&state).cloned()
    })();
    username.zeroize();
    password.zeroize();
    result
}

#[tauri::command]
fn logout(state: State<'_, Store>) -> Result<(), String> {
    state
        .0
        .lock()
        .map_err(|_| "Account store is unavailable")?
        .active = None;
    Ok(())
}

#[tauri::command]
fn generate_session(
    request: GenerateRequest,
    state: State<'_, Store>,
) -> Result<SessionPlan, String> {
    let state = state.0.lock().map_err(|_| "Account store is unavailable")?;
    generate(&request, active_data(&state)?)
}

#[tauri::command]
fn load_app_data(state: State<'_, Store>) -> Result<AppData, String> {
    let state = state.0.lock().map_err(|_| "Account store is unavailable")?;
    active_data(&state).cloned()
}

#[tauri::command]
fn save_settings(
    settings: model::Settings,
    state: State<'_, Store>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut state = state.0.lock().map_err(|_| "Account store is unavailable")?;
    active_data_mut(&mut state)?.settings = settings;
    save(&app, &state.db)
}

#[tauri::command]
fn record_session(
    record: SessionRecord,
    state: State<'_, Store>,
    app: tauri::AppHandle,
) -> Result<AppData, String> {
    let mut state = state.0.lock().map_err(|_| "Account store is unavailable")?;
    let data = active_data_mut(&mut state)?;
    data.absorb(record);
    let result = data.clone();
    save(&app, &state.db)?;
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db = load(app.handle()).map_err(std::io::Error::other)?;
            app.manage(Store(Mutex::new(AuthState {
                db,
                active: None,
                failed_logins: 0,
                retry_after: None,
            })));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            account_status,
            create_account,
            login,
            logout,
            generate_session,
            load_app_data,
            save_settings,
            record_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running KeyPrac");
}
