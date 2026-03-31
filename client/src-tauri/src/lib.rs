mod screen_capture;
mod input_controller;
mod config_manager;

use std::sync::Arc;
use tauri::{State, Emitter, Manager};
use screen_capture::ScreenCapture;
use input_controller::InputController;
use config_manager::ConfigManager;

struct AppState {
    screen_capture: Arc<ScreenCapture>,
    input_controller: Arc<InputController>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn start_screen_capture(state: State<'_, AppState>, app_handle: tauri::AppHandle) -> Result<(), String> {
    let capture = state.screen_capture.clone();
    capture.start_capture(move |base64_frame| {
        let _ = app_handle.emit("screen-frame", base64_frame);
    }).await
}

#[tauri::command]
async fn stop_screen_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.screen_capture.stop_capture().await;
    Ok(())
}

#[tauri::command]
fn handle_mouse_move(state: State<'_, AppState>, x: f64, y: f64) {
    state.input_controller.mouse_move(x, y);
}

#[tauri::command]
fn handle_mouse_click(state: State<'_, AppState>, button: &str, is_down: bool) {
    state.input_controller.mouse_click(button, is_down);
}

#[tauri::command]
fn handle_mouse_scroll(state: State<'_, AppState>, x: i32, y: i32) {
    state.input_controller.mouse_scroll(x, y);
}

#[tauri::command]
fn handle_key_event(state: State<'_, AppState>, key_code: &str, is_down: bool) {
    state.input_controller.key_event(key_code, is_down);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        screen_capture: Arc::new(ScreenCapture::new()),
        input_controller: Arc::new(InputController::new()),
    };

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let config_manager = Arc::new(ConfigManager::new(app.handle()));
            
            let config_manager_clone = config_manager.clone();
            tauri::async_runtime::spawn(async move {
                config_manager_clone.try_fetch_remote_config().await;
            });
            
            app.manage(config_manager);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            start_screen_capture,
            stop_screen_capture,
            handle_mouse_move,
            handle_mouse_click,
            handle_mouse_scroll,
            handle_key_event,
            config_manager::get_config,
            config_manager::get_servers,
            config_manager::update_config,
            config_manager::refresh_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
