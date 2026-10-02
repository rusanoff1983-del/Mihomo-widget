#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use reqwest::{Client, Method};
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{menu::{Menu, MenuItem}, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_updater::UpdaterExt;
#[cfg(not(debug_assertions))]
use tauri_plugin_autostart::ManagerExt as AutostartExt;

mod settings;
use settings::SettingsState;

pub(crate) fn is_theme(theme: &str) -> bool {
    matches!(theme, "dark" | "light" | "cocoa" | "sage" | "nocturne" | "dune" | "lagoon" | "crimson" | "jade" | "velvet" | "amber" | "sakura" | "lumiere" | "noir-amber" | "violet-ink" | "moss-noir")
}

struct FrameTheme(Mutex<String>);

// Keep the native border so Windows can round the resizable window corners.
// DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWA_BORDER_COLOR = 34.
#[cfg(windows)]
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(hwnd: *mut std::ffi::c_void, attribute: u32, value: *const u32, size: u32) -> i32;
}

#[cfg(windows)]
fn apply_native_frame(hwnd: *mut std::ffi::c_void, theme: &str) {
    let rounded: u32 = 2; // DWMWCP_ROUND
    let color: u32 = match theme {
        "light" => 0x00EC_EEEF,
        "cocoa" => 0x001A_1C21,
        "sage" => 0x001D_211D,
        "nocturne" => 0x0026_101A,
        "dune" => 0x00C7_D8EA,
        "lagoon" => 0x0036_2B00,
        "crimson" => 0x0012_0A2B,
        "jade" => 0x0028_2E1B,
        "velvet" => 0x0031_131A,
        "amber" => 0x0010_1721,
        "sakura" => 0x001A_0E0B,
        "lumiere" => 0x0022_1A15,
        "noir-amber" => 0x001D_1817,
        "violet-ink" => 0x0010_0B0C,
        "moss-noir" => 0x0011_1511,
        _ => 0x001B_1B1B,
    };
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, 33, &rounded, std::mem::size_of::<u32>() as u32);
        let _ = DwmSetWindowAttribute(hwnd, 34, &color, std::mem::size_of::<u32>() as u32);
    }
}

#[tauri::command]
fn set_window_theme(window: tauri::WebviewWindow, theme: String, state: State<'_, FrameTheme>) {
    let theme = if is_theme(&theme) { theme } else { "dark".into() };
    if let Ok(mut current) = state.0.lock() { *current = theme.clone(); }
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() { apply_native_frame(hwnd.0, &theme); }
    #[cfg(not(windows))]
    let _ = window;
}

struct ApiClient(Client);

fn endpoint(host: &str, path: &str) -> String {
    format!("{}{}", host.trim_end_matches('/'), path)
}

async fn request(client: &Client, host: &str, secret: &str, path: &str, method: Method, body: Option<Value>) -> Result<Value, String> {
    let mut req = client.request(method, endpoint(host, path)).bearer_auth(secret);
    if let Some(json) = body { req = req.json(&json); }
    let response = req.send().await.map_err(|e| format!("Не удалось подключиться: {e}"))?;
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() { return Err(format!("HTTP {}: {}", status.as_u16(), text)); }
    if text.trim().is_empty() { Ok(Value::Object(Default::default())) }
    else { serde_json::from_str(&text).map_err(|e| format!("Некорректный ответ Mihomo: {e}")) }
}

#[tauri::command]
async fn mihomo_request(host: String, secret: String, path: String, method: String, body: Option<Value>, state: State<'_, ApiClient>) -> Result<Value, String> {
    let method = Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
    request(&state.0, &host, &secret, &path, method, body).await
}

#[tauri::command]
async fn traffic_sample(host: String, secret: String, state: State<'_, ApiClient>) -> Result<Value, String> {
    let response = state.0.get(endpoint(&host, "/traffic")).bearer_auth(secret).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() { return Err(format!("HTTP {}", response.status().as_u16())); }
    let mut response = response;
    let chunk = tokio::time::timeout(Duration::from_secs(3), response.chunk()).await
        .map_err(|_| "Таймаут чтения скорости".to_string())?.map_err(|e| e.to_string())?;
    let text = chunk.map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
    text.lines().filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok()).last()
        .ok_or_else(|| "Нет данных скорости".to_string())
}

#[tauri::command]
async fn check_updates(app: tauri::AppHandle, state: State<'_, ApiClient>) -> Result<Option<Value>, String> {
    let response = state.0.get("https://api.github.com/repos/rusanoff1983-del/Mihomo-widget/releases/latest")
        .header("User-Agent", "Mihomo-Widget")
        .send().await.map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND { return Ok(None); }
    if !response.status().is_success() { return Err(format!("GitHub HTTP {}", response.status())); }
    let release: Value = response.json().await.map_err(|e| e.to_string())?;
    let version = release.get("tag_name").and_then(Value::as_str).unwrap_or("").trim_start_matches('v');
    let available = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?.is_some();
    Ok(Some(serde_json::json!({"version": version, "available": available,
        "canInstall": available && !cfg!(debug_assertions),
        "url": release.get("html_url").and_then(Value::as_str)})))
}

#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    if cfg!(debug_assertions) { return Err("Install updates from the packaged application".into()); }
    let Some(update) = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())? else {
        return Err("No update available".into());
    };
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) { settings::capture_now(&window); let _ = window.hide(); }

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") { settings::capture_now(&win); }
    app.exit(0);
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) { settings::capture_now(&win); let _ = win.hide(); }
        else { let _ = win.show(); let _ = win.set_focus(); }
    }
}

#[cfg(windows)]
fn system_english() -> bool {
    #[link(name = "kernel32")]
    unsafe extern "system" { fn GetUserDefaultLocaleName(buffer: *mut u16, capacity: i32) -> i32; }
    let mut buffer = [0u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    if length <= 1 { return false; }
    !String::from_utf16_lossy(&buffer[..(length as usize - 1)]).to_lowercase().starts_with("ru")
}
#[cfg(not(windows))]
fn system_english() -> bool { true }

fn tray_text(app: &tauri::AppHandle, ru: &str, en: &str) -> String {
    let state = app.state::<SettingsState>();
    let english = state.data.lock().map(|s| s.language == "en" || (s.language == "system" && system_english())).unwrap_or(false);
    if english { en.into() } else { ru.into() }
}

pub(crate) fn refresh_tray_menu(app: &tauri::AppHandle) {
    let Ok(show) = MenuItem::with_id(app, "show", tray_text(app, "Показать / скрыть", "Show / hide"), true, None::<&str>) else { return; };
    let Ok(settings_item) = MenuItem::with_id(app, "settings", tray_text(app, "Настройки", "Settings"), true, None::<&str>) else { return; };
    let state = app.state::<SettingsState>();
    let (top, click) = state.data.lock().map(|s| (s.always_on_top, s.click_through)).unwrap_or((true, false));
    let top_text = if top { tray_text(app, "✓ Поверх всех окон", "✓ Always on top") } else { tray_text(app, "Поверх всех окон", "Always on top") };
    let click_text = if click { tray_text(app, "✓ Клик насквозь", "✓ Click-through") } else { tray_text(app, "Клик насквозь", "Click-through") };
    let Ok(top_item) = MenuItem::with_id(app, "top", top_text, true, None::<&str>) else { return; };
    let Ok(click_item) = MenuItem::with_id(app, "click", click_text, true, None::<&str>) else { return; };
    let Ok(quit) = MenuItem::with_id(app, "quit", tray_text(app, "Выход", "Quit"), true, None::<&str>) else { return; };
    let Ok(menu) = Menu::with_items(app, &[&show, &settings_item, &top_item, &click_item, &quit]) else { return; };
    if let Some(tray) = app.tray_by_id("main-tray") { let _ = tray.set_menu(Some(menu)); }
}

fn toggle_tray_setting(app: &tauri::AppHandle, click: bool) {
    let Some(win) = app.get_webview_window("main") else { return; };
    let state = app.state::<SettingsState>();
    let mut data = match state.data.lock() { Ok(data) => data, Err(_) => return };
    if click {
        let next = !data.click_through;
        if win.set_ignore_cursor_events(next).is_ok() { data.click_through = next; }
    } else {
        let next = !data.always_on_top;
        if win.set_always_on_top(next).is_ok() { data.always_on_top = next; }
    }
    let _ = app.emit("widget-settings-changed", (data.always_on_top, data.click_through));
    drop(data);
    let _ = state.save();
    refresh_tray_menu(app);
}

pub fn run() {
    let client = Client::builder().timeout(Duration::from_secs(12)).build().expect("HTTP client");
    tauri::Builder::default()
        .manage(ApiClient(client))
        .manage(FrameTheme(Mutex::new("dark".into())))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") { let _ = win.show(); let _ = win.set_focus(); }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, _shortcut, event| {
            if event.state == ShortcutState::Pressed { toggle_window(app); }
        }).build())
        .invoke_handler(tauri::generate_handler![mihomo_request, traffic_sample, check_updates, install_update, hide_window, quit_app, set_window_theme,
            settings::get_settings, settings::update_settings, settings::set_always_on_top, settings::set_click_through,
            settings::get_autostart, settings::set_autostart])
        .setup(|app| {
            let settings = SettingsState::new(app.handle()).map_err(std::io::Error::other)?;
            app.manage(settings);
            #[cfg(not(debug_assertions))]
            if app.state::<SettingsState>().data.lock().map(|s| s.autostart).unwrap_or(false) {
                let _ = app.autolaunch().enable();
            }
            let show = MenuItem::with_id(app, "show", "Показать / скрыть", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let icon = tauri::image::Image::from_bytes(include_bytes!("../../tray_icon.png"))?;
            TrayIconBuilder::with_id("main-tray").icon(icon).tooltip("Mihomo Widget")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => toggle_window(app),
                    "settings" => { if let Some(win) = app.get_webview_window("main") { let _ = win.show(); let _ = win.set_focus(); let _ = app.emit("open-widget-settings", ()); } },
                    "top" => toggle_tray_setting(app, false),
                    "click" => toggle_tray_setting(app, true),
                    "quit" => { if let Some(win) = app.get_webview_window("main") { settings::capture_now(&win); } app.exit(0); },
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        toggle_window(tray.app_handle());
                    }
                }).build(app)?;
            refresh_tray_menu(app.handle());
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(windows)]
                if let Ok(hwnd) = window.hwnd() { apply_native_frame(hwnd.0, "dark"); }
                settings::restore_window(&window, &app.state::<SettingsState>());
            }
            let _ = app.global_shortcut().register("Ctrl+Shift+Space");
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(windows)]
            if matches!(event, tauri::WindowEvent::Focused(_) | tauri::WindowEvent::Resized(_)) {
                if let Ok(hwnd) = window.hwnd() {
                    let theme = window.app_handle().state::<FrameTheme>();
                    if let Ok(current) = theme.0.lock() { apply_native_frame(hwnd.0, &current); };
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(webview) = window.app_handle().get_webview_window(window.label()) { settings::capture_now(&webview); }
                let _ = window.hide();
            }
            if matches!(event, tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)) {
                if let Some(webview) = window.app_handle().get_webview_window(window.label()) {
                    settings::window_changed(&webview, matches!(event, tauri::WindowEvent::Moved(_)));
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Proxy Widget");
}
