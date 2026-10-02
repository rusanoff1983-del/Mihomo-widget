use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, path::PathBuf, sync::{atomic::{AtomicU64, Ordering}, Mutex}, time::Duration};
use tauri::{AppHandle, Manager, PhysicalPosition, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt as AutostartExt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub monitor_name: String,
    pub monitor_x: i32,
    pub monitor_y: i32,
}
impl Default for Placement {
    fn default() -> Self { Self { x: 0, y: 0, width: 520, height: 540, monitor_name: String::new(), monitor_x: 0, monitor_y: 0 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u8,
    pub initialized: bool,
    pub language: String,
    pub theme: String,
    pub transparency: u8,
    pub show_flags: bool,
    pub color_ping: bool,
    pub always_on_top: bool,
    pub edge_snap: bool,
    pub click_through: bool,
    pub autostart: bool,
    pub host: String,
    pub pinned: Vec<String>,
    pub current_group: String,
    pub placement: Option<Placement>,
}
impl Default for Settings {
    fn default() -> Self {
        Self { version: 1, initialized: false, language: "system".into(), theme: "dark".into(), transparency: 0,
            show_flags: false, color_ping: false, always_on_top: true, edge_snap: true,
            click_through: false, autostart: false, host: String::new(), pinned: vec![],
            current_group: "PC".into(), placement: None }
    }
}

pub struct SettingsState {
    pub data: Mutex<Settings>,
    path: PathBuf,
    movement_seq: AtomicU64,
}
impl SettingsState {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("settings.json");
        let loaded = fs::read_to_string(&path).ok().and_then(|raw| serde_json::from_str(&raw).ok());
        let data = loaded.unwrap_or_default();
        Ok(Self { data: Mutex::new(data), path, movement_seq: AtomicU64::new(0) })
    }
    pub fn save(&self) -> Result<(), String> {
        let data = self.data.lock().map_err(|e| e.to_string())?.clone();
        let raw = serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?;
        fs::write(&self.path, raw).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Result<Option<Settings>, String> {
    let data = state.data.lock().map_err(|e| e.to_string())?.clone();
    if !data.initialized { return Ok(None); }
    Ok(Some(data))
}

#[tauri::command]
pub fn update_settings(app: AppHandle, patch: Value, state: State<'_, SettingsState>) -> Result<Settings, String> {
    let mut data = state.data.lock().map_err(|e| e.to_string())?;
    let obj = patch.as_object().ok_or("Invalid settings patch")?;
    for (key, value) in obj {
        match key.as_str() {
            "language" => if let Some(v) = value.as_str() { if ["system", "ru", "en"].contains(&v) { data.language = v.into(); } },
            "theme" => if let Some(v) = value.as_str() { if super::is_theme(v) || v == "system" { data.theme = v.into(); } },
            "transparency" => if let Some(v) = value.as_u64() { data.transparency = v.min(60) as u8; },
            "showFlags" => if let Some(v) = value.as_bool() { data.show_flags = v; },
            "colorPing" => if let Some(v) = value.as_bool() { data.color_ping = v; },
            "edgeSnap" => if let Some(v) = value.as_bool() { data.edge_snap = v; },
            "host" => if let Some(v) = value.as_str() { if v.len() <= 512 { data.host = v.into(); } },
            "pinned" => if let Some(v) = value.as_array() { data.pinned = v.iter().filter_map(Value::as_str).take(100).map(str::to_owned).collect(); },
            "currentGroup" => if let Some(v) = value.as_str() { if v.len() <= 200 { data.current_group = v.into(); } },
            _ => {},
        }
    }
    data.initialized = true;
    let saved = data.clone();
    drop(data);
    state.save()?;
    if obj.contains_key("language") { super::refresh_tray_menu(&app); }
    Ok(saved)
}

#[tauri::command]
pub fn set_always_on_top(window: WebviewWindow, enabled: bool, state: State<'_, SettingsState>) -> Result<(), String> {
    window.set_always_on_top(enabled).map_err(|e| e.to_string())?;
    state.data.lock().map_err(|e| e.to_string())?.always_on_top = enabled;
    state.save()?;
    super::refresh_tray_menu(window.app_handle());
    Ok(())
}

#[tauri::command]
pub fn set_click_through(window: WebviewWindow, enabled: bool, state: State<'_, SettingsState>) -> Result<(), String> {
    window.set_ignore_cursor_events(enabled).map_err(|e| e.to_string())?;
    state.data.lock().map_err(|e| e.to_string())?.click_through = enabled;
    state.save()?;
    super::refresh_tray_menu(window.app_handle());
    Ok(())
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool, state: State<'_, SettingsState>) -> Result<(), String> {
    if enabled { app.autolaunch().enable().map_err(|e| e.to_string())?; }
    else { app.autolaunch().disable().map_err(|e| e.to_string())?; }
    state.data.lock().map_err(|e| e.to_string())?.autostart = enabled;
    state.save()
}

pub fn restore_window(window: &WebviewWindow, state: &SettingsState) {
    let settings = match state.data.lock() { Ok(s) => s.clone(), Err(_) => return };
    let _ = window.set_always_on_top(settings.always_on_top);
    // Click-through is deliberately reset on launch so the window is always recoverable.
    let _ = window.set_ignore_cursor_events(false);
    if settings.click_through {
        if let Ok(mut data) = state.data.lock() { data.click_through = false; }
        let _ = state.save();
    }
    if let Some(saved) = settings.placement {
        let _ = window.set_size(tauri::PhysicalSize::new(saved.width.max(420), saved.height.max(420)));
        if let Ok(monitors) = window.available_monitors() {
            let monitor = monitors.iter().find(|m| m.name().map(|n| n.as_str()) == Some(saved.monitor_name.as_str()) && m.position().x == saved.monitor_x && m.position().y == saved.monitor_y)
                .or_else(|| monitors.iter().find(|m| m.name().map(|n| n.as_str()) == Some(saved.monitor_name.as_str())))
                .or_else(|| monitors.first());
            if let Some(monitor) = monitor {
                let x = monitor.position().x + saved.x - saved.monitor_x;
                let y = monitor.position().y + saved.y - saved.monitor_y;
                let _ = window.set_position(PhysicalPosition::new(x, y));
                clamp_to_work_area(window);
                return;
            }
        }
    }
    if let Ok(Some(monitor)) = window.current_monitor() {
        let origin = monitor.position();
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let x = origin.x + size.width as i32 - (536.0 * scale) as i32;
        let y = origin.y + (40.0 * scale) as i32;
        let _ = window.set_position(PhysicalPosition::new(x, y));
        clamp_to_work_area(window);
    }
}

fn capture_placement(window: &WebviewWindow, state: &SettingsState) {
    let (Ok(pos), Ok(size), Ok(Some(monitor))) = (window.outer_position(), window.outer_size(), window.current_monitor()) else { return; };
    let placement = Placement { x: pos.x, y: pos.y, width: size.width, height: size.height,
        monitor_name: monitor.name().cloned().unwrap_or_default(), monitor_x: monitor.position().x, monitor_y: monitor.position().y };
    if let Ok(mut data) = state.data.lock() { data.placement = Some(placement); }
    let _ = state.save();
}

pub fn capture_now(window: &WebviewWindow) {
    let state = window.app_handle().state::<SettingsState>();
    capture_placement(window, &state);
}

pub fn window_changed(window: &WebviewWindow, moved: bool) {
    let state = window.app_handle().state::<SettingsState>();
    if moved && state.data.lock().map(|s| s.edge_snap).unwrap_or(false) { snap_to_work_area(window); }
    let seq = state.movement_seq.fetch_add(1, Ordering::Relaxed) + 1;
    let app = window.app_handle().clone();
    let win = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(350));
        let state = app.state::<SettingsState>();
        if state.movement_seq.load(Ordering::Relaxed) == seq { capture_placement(&win, &state); }
    });
}

fn snap_to_work_area(window: &WebviewWindow) {
    let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else { return; };
    let Some((left, top, right, bottom)) = work_area(window) else { return; };
    let x_right = right - size.width as i32;
    let y_bottom = bottom - size.height as i32;
    let x = if (pos.x - left).abs() <= 18 { left } else if (pos.x - x_right).abs() <= 18 { x_right } else { pos.x };
    let y = if (pos.y - top).abs() <= 18 { top } else if (pos.y - y_bottom).abs() <= 18 { y_bottom } else { pos.y };
    if x != pos.x || y != pos.y { let _ = window.set_position(PhysicalPosition::new(x, y)); }
}

fn clamp_to_work_area(window: &WebviewWindow) {
    let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else { return; };
    let Some((left, top, right, bottom)) = work_area(window) else { return; };
    let x = pos.x.clamp(left, (right - size.width as i32).max(left));
    let y = pos.y.clamp(top, (bottom - size.height as i32).max(top));
    if x != pos.x || y != pos.y { let _ = window.set_position(PhysicalPosition::new(x, y)); }
}

#[cfg(windows)]
fn work_area(window: &WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    use std::ffi::c_void;
    #[repr(C)] struct Rect { left: i32, top: i32, right: i32, bottom: i32 }
    #[repr(C)] struct MonitorInfo { size: u32, monitor: Rect, work: Rect, flags: u32 }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MonitorFromWindow(hwnd: *mut c_void, flags: u32) -> *mut c_void;
        fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
    }
    let hwnd = window.hwnd().ok()?.0;
    let monitor = unsafe { MonitorFromWindow(hwnd, 2) };
    if monitor.is_null() { return None; }
    let mut info = MonitorInfo { size: std::mem::size_of::<MonitorInfo>() as u32,
        monitor: Rect { left: 0, top: 0, right: 0, bottom: 0 }, work: Rect { left: 0, top: 0, right: 0, bottom: 0 }, flags: 0 };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 { return None; }
    Some((info.work.left, info.work.top, info.work.right, info.work.bottom))
}
#[cfg(not(windows))]
fn work_area(window: &WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    let monitor = window.current_monitor().ok()??;
    let p = monitor.position(); let s = monitor.size();
    Some((p.x, p.y, p.x + s.width as i32, p.y + s.height as i32))
}
