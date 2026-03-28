mod screen_capture;
mod input_controller;

use std::sync::Arc;
use tauri::{State, Manager, Emitter};
use screen_capture::ScreenCapture;
use input_controller::InputController;

// 存储全局状态
struct AppState {
    screen_capture: Arc<ScreenCapture>,
    input_controller: Arc<InputController>,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// 启动屏幕采集测试命令
#[tauri::command]
async fn start_screen_capture(state: State<'_, AppState>, app_handle: tauri::AppHandle) -> Result<(), String> {
    let capture = state.screen_capture.clone();
    
    // 启动采集，并将每一帧通过 Tauri Event 发送给前端
    capture.start_capture(move |base64_frame| {
        let _ = app_handle.emit("screen-frame", base64_frame);
    }).await
}

// 停止屏幕采集测试命令
#[tauri::command]
async fn stop_screen_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.screen_capture.stop_capture().await;
    Ok(())
}

// 接收前端传来的鼠标移动指令
#[tauri::command]
fn handle_mouse_move(state: State<'_, AppState>, x: i32, y: i32) {
    state.input_controller.mouse_move(x, y);
}

// 接收前端传来的鼠标点击指令
#[tauri::command]
fn handle_mouse_click(state: State<'_, AppState>, button: &str, is_down: bool) {
    state.input_controller.mouse_click(button, is_down);
}

// 接收前端传来的鼠标滚轮指令
#[tauri::command]
fn handle_mouse_scroll(state: State<'_, AppState>, x: i32, y: i32) {
    state.input_controller.mouse_scroll(x, y);
}

// 接收前端传来的键盘指令
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
        .invoke_handler(tauri::generate_handler![
            greet,
            start_screen_capture,
            stop_screen_capture,
            handle_mouse_move,
            handle_mouse_click,
            handle_mouse_scroll,
            handle_key_event
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
