#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, os::unix::fs::PermissionsExt, sync::Arc};

use tokio::{
  fs,
  process::Command,
  sync::Mutex,
  time::{Duration, MissedTickBehavior},
};
use serde::{Serialize, Deserialize};

use tauri::{
  image::Image, menu::{Menu, MenuItem}, tray::{TrayIcon, TrayIconBuilder}, App, AppHandle, Emitter, Manager, State
};

const WG_SCRIPT: &str = include_str!("../scripts/wg.sh");

const WG_ZENITY_SCRIPT: &str = include_str!("../scripts/zenity.sh");

const APP_TITLE: &str = "Wireguard GUI";
const APP_STATE_CHANGED_EVENT: &str = "app-state-changed";
const NM_CONN_PREFIX: &str = "wg-gui-";

const TRAY_CONNECTED_ICON: &[u8] =
  include_bytes!("../icons/tray_connected.png");
const TRAY_DISCONNECTED_ICON: &[u8] = include_bytes!("../icons/tray.png");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpPayload {
  pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePartial {
  pub name: String,
  pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum ConnSt {
  Connected,
  Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppStInner {
  pub conn_st: ConnSt,
  pub conf_dir: String,
  pub current: Option<String>,
  pub pub_ip: Option<String>,
  pub profiles: Vec<Profile>,
}

impl Default for AppStInner {
  fn default() -> Self {
    let home = std::env::var("HOME").unwrap();
    Self {
      conn_st: ConnSt::Disconnected,
      conf_dir: format!("{home}/.config/wireguard-gui"),
      current: None,
      pub_ip: None,
      profiles: vec![],
    }
  }
}

unsafe impl Send for AppStInner {}

#[derive(Clone, Debug)]
struct AppSt(Arc<Mutex<AppStInner>>);

unsafe impl Send for AppSt {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppError {
  message: String,
}

unsafe impl Send for AppError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
  pub name: String,
  pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
  pub success: Vec<String>,
  pub failed: Vec<ImportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
  pub file_name: String,
  pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
  pub success: Vec<String>,
  pub failed: Vec<ExportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportError {
  pub profile_name: String,
  pub error: String,
}

/// We create 2 scripts one to open a popup to allow root
/// And the other to execute wg-quick as root with the provided config
async fn create_scripts(conf_dir: &str) {
  std::fs::create_dir_all(format!("{conf_dir}/profiles")).unwrap();
  let wg_path = format!("{conf_dir}/wg.sh");
  std::fs::write(&wg_path, WG_SCRIPT).unwrap();
  std::fs::set_permissions(&wg_path, std::fs::Permissions::from_mode(0o700))
    .unwrap();
  let zenity_path = format!("{conf_dir}/zenity.sh");
  fs::write(&zenity_path, WG_ZENITY_SCRIPT).await.unwrap();
  fs::set_permissions(&zenity_path, std::fs::Permissions::from_mode(0o700))
    .await
    .unwrap();
}

fn profile_from_nm_conn_name(name: &str) -> Option<String> {
  Some(
    name.strip_prefix(NM_CONN_PREFIX)
      .unwrap_or(name)
      .to_owned(),
  )
}

async fn get_active_nm_profile(current_hint: Option<&str>) -> Option<String> {
  println!(
    "[wg-gui] get_active_nm_profile: current_hint={:?}",
    current_hint,
  );
  let output = Command::new("nmcli")
    .args(["-t", "-f", "NAME,TYPE", "connection", "show", "--active"])
    .output()
    .await
    .ok()?;
  if !output.status.success() {
    println!(
      "[wg-gui] get_active_nm_profile: nmcli failed status={:?} stderr={}",
      output.status.code(),
      String::from_utf8_lossy(&output.stderr),
    );
    return None;
  }
  println!(
    "[wg-gui] get_active_nm_profile: stdout={} stderr={}",
    String::from_utf8_lossy(&output.stdout).trim(),
    String::from_utf8_lossy(&output.stderr).trim(),
  );
  let active_wireguard_names = String::from_utf8(output.stdout)
    .ok()?
    .lines()
    .filter_map(|line| {
      let (name, conn_type) = line.rsplit_once(':')?;
      let conn_type = conn_type.trim().to_ascii_lowercase();
      if conn_type != "wireguard" && conn_type != "vpn" {
        return None;
      }
      Some(name.to_owned())
    })
    .collect::<Vec<_>>();
  println!(
    "[wg-gui] get_active_nm_profile: filtered_active_names={:?}",
    active_wireguard_names,
  );

  if let Some(current) = current_hint {
    let prefixed = format!("{NM_CONN_PREFIX}{current}");
    if active_wireguard_names.iter().any(|name| name == current || name == &prefixed) {
      println!(
        "[wg-gui] get_active_nm_profile: matched current_hint={} prefixed={}",
        current,
        prefixed,
      );
      return Some(current.to_owned());
    }

    println!(
      "[wg-gui] get_active_nm_profile: no active match for current_hint={} prefixed={}",
      current,
      prefixed,
    );
    return None;
  }

  let detected = active_wireguard_names
    .iter()
    .find_map(|name| profile_from_nm_conn_name(name));
  println!(
    "[wg-gui] get_active_nm_profile: detected_without_hint={:?}",
    detected,
  );
  detected
}

async fn read_current_profile(conf_dir: &str) -> Option<String> {
  fs::read_to_string(format!("{conf_dir}/current"))
    .await
    .ok()
    .map(|value| value.trim().to_owned())
    .filter(|value| !value.is_empty())
}

async fn detect_connection_state(
  current_hint: Option<String>,
  conf_dir: &str,
) -> (ConnSt, Option<String>) {
  println!(
    "[wg-gui] detect_connection_state: is_snap={} current_hint={:?} conf_dir={}",
    std::env::var_os("IS_SNAP").is_some(),
    current_hint,
    conf_dir,
  );
  if std::env::var_os("IS_SNAP").is_some() {
    let snap_hint = match current_hint {
      Some(current) => Some(current),
      None => read_current_profile(conf_dir).await,
    };
    println!(
      "[wg-gui] detect_connection_state: snap_hint={:?}",
      snap_hint,
    );

    if let Some(profile) = get_active_nm_profile(snap_hint.as_deref()).await {
      println!(
        "[wg-gui] detect_connection_state: snap connected profile={}",
        profile,
      );
      return (ConnSt::Connected, Some(profile));
    }

    println!("[wg-gui] detect_connection_state: snap disconnected");
    return (ConnSt::Disconnected, None);
  }

  let current = match current_hint {
    Some(current) => Some(current),
    None => read_current_profile(conf_dir).await,
  };
  println!(
    "[wg-gui] detect_connection_state: non-snap current={:?}",
    current,
  );

  if let Some(current_name) = current.as_deref()
    && get_con_st(current_name).await == ConnSt::Connected {
      println!(
        "[wg-gui] detect_connection_state: interface connected current={}",
        current_name,
      );
      return (ConnSt::Connected, Some(current_name.to_owned()));
  }

  // Fallback: check NM for active WireGuard connections (handles both
  // wg-gui-<name> prefixed and exact-name connections)
  if let Some(profile) = get_active_nm_profile(current.as_deref()).await {
    println!(
      "[wg-gui] detect_connection_state: NM fallback connected profile={}",
      profile,
    );
    return (ConnSt::Connected, Some(profile));
  }

  println!("[wg-gui] detect_connection_state: disconnected");
  (ConnSt::Disconnected, None)
}

async fn sync_connection_state(
  app: &AppHandle,
  app_state: &AppSt,
) -> Result<(), AppError> {
  let (conf_dir, previous) = {
    let s = app_state.0.lock().await;
    (
      s.conf_dir.clone(),
      (s.conn_st.clone(), s.current.clone(), s.pub_ip.clone()),
    )
  };
  let (prev_conn_st, prev_current, prev_pub_ip) = previous;
  println!(
    "[wg-gui] sync_connection_state: previous conn_st={:?} current={:?} pub_ip={:?}",
    prev_conn_st,
    prev_current,
    prev_pub_ip,
  );

  let (next_conn_st, next_current) = detect_connection_state(prev_current.clone(), &conf_dir).await;
  let state_changed = next_conn_st != prev_conn_st || next_current != prev_current;
  let next_pub_ip = if next_conn_st == ConnSt::Connected {
    if state_changed || prev_pub_ip.is_none() {
      get_pub_ip().await.ok()
    } else {
      prev_pub_ip.clone()
    }
  } else if state_changed {
    get_pub_ip().await.ok()
  } else {
    prev_pub_ip.clone()
  };
  println!(
    "[wg-gui] sync_connection_state: next conn_st={:?} current={:?} pub_ip={:?} state_changed={}",
    next_conn_st,
    next_current,
    next_pub_ip,
    state_changed,
  );

  if !state_changed && next_pub_ip == prev_pub_ip {
    println!("[wg-gui] sync_connection_state: no update needed");
    return Ok(());
  }

  {
    let mut s = app_state.0.lock().await;
    s.conn_st = next_conn_st.clone();
    s.current = next_current.clone();
    s.pub_ip = next_pub_ip;
  }

  if let Some(current) = next_current {
    let _ = fs::write(format!("{conf_dir}/current"), current.trim()).await;
  } else {
    let _ = fs::remove_file(format!("{conf_dir}/current")).await;
  }

  if let Some(tray) = app.tray_by_id("main") {
    let icon = if next_conn_st == ConnSt::Connected {
      TRAY_CONNECTED_ICON
    } else {
      TRAY_DISCONNECTED_ICON
    };
    println!(
      "[wg-gui] sync_connection_state: updating tray icon for conn_st={:?}",
      next_conn_st,
    );
    let _ = tray.set_icon(Some(Image::from_bytes(icon).unwrap()));
  }

  let payload = app_state.0.lock().await.clone();
  println!(
    "[wg-gui] sync_connection_state: emitting payload conn_st={:?} current={:?} pub_ip={:?}",
    payload.conn_st,
    payload.current,
    payload.pub_ip,
  );
  let _ = app.emit(APP_STATE_CHANGED_EVENT, payload);
  Ok(())
}

async fn get_con_st(current: &str) -> ConnSt {
  let output = Command::new("ip")
    .args(["-br", "link", "show", "dev", current])
    .output()
    .await
    .expect("ip command failed");
  println!(
    "[wg-gui] get_con_st: current={} status={:?} stdout={} stderr={}",
    current,
    output.status.code(),
    String::from_utf8_lossy(&output.stdout).trim(),
    String::from_utf8_lossy(&output.stderr).trim(),
  );
  // check status code
  if output.status.success() {
    return ConnSt::Connected;
  }
  ConnSt::Disconnected
}

async fn init_app_st() -> AppSt {
  let default_state = AppStInner::default();
  let conf_dir = default_state.conf_dir.clone();
  let current = read_current_profile(&conf_dir).await;
  let app_state = AppSt(Arc::new(Mutex::new(default_state)));
  create_scripts(&conf_dir).await;
  let (conn_st, current) = detect_connection_state(current, &conf_dir).await;
  let mut s = app_state.0.lock().await;
  s.pub_ip = (get_pub_ip().await).ok();
  s.conn_st = conn_st;
  s.current = current;
  if s.current.is_none() {
    let _ = fs::remove_file(format!("{}/current", s.conf_dir)).await;
  }
  app_state.clone()
}

async fn exec_wg(app_state: &AppSt, profile: &str) -> Result<(), AppError> {
  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let mut envs = HashMap::new();
  envs.insert("PROFILE".to_owned(), profile);
  if std::env::var_os("IS_SNAP").is_some() {
    envs.insert("IS_SNAP".to_owned(), "true");
    println!("Running in snap environment, setting IS_SNAP env variable for wg.sh");
  }
  let res = Command::new("bash")
    .args([format!("{conf_dir}/wg.sh")])
    .envs(envs)
    .output()
    .await
    .expect("failed to execute process");
  if res.status.code().unwrap_or_default() != 0 {
    return Err(AppError {
      message: String::from_utf8(res.stderr).unwrap_or_default(),
    });
  }
  Ok(())
}

async fn get_pub_ip() -> Result<String, AppError> {
  let payload = reqwest::get("https://httpbin.org/ip")
    .await
    .map_err(|err| AppError {
      message: err.to_string(),
    })?
    .json::<IpPayload>()
    .await.map_err(|err| AppError {
      message: err.to_string(),
    })?;
  Ok(payload.origin)
}

#[tauri::command]
async fn get_state(
  app_state: State<'_, AppSt>,
) -> Result<AppStInner, AppError> {
  Ok(app_state.0.lock().await.clone())
}

#[tauri::command]
async fn create_profile(
  app_state: State<'_, AppSt>,
  new_profile: ProfilePartial,
) -> Result<(), AppError> {
  let s = app_state.0.lock().await.clone();
  // allow only alphanumerac
  let name = new_profile.name;
  // allow alphanumeric and - and _
  if !name
    .chars()
    .all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
      return Err(AppError {
        message: "Name must only containt alphanumeric values, - or _".to_owned(),
      });
  }
  let path = format!("{}/profiles/{name}.conf", s.conf_dir);
  if fs::try_exists(&path).await.unwrap() {
    return Err(AppError {
      message: "Profile already exist".into(),
    });
  }
  fs::write(path, new_profile.content).await.unwrap();
  Ok(())
}

#[tauri::command]
async fn delete_profile(
  app: AppHandle,
  app_state: State<'_, AppSt>,
  profile_name: String,
) -> Result<(), AppError> {
  let s = app_state.0.lock().await.clone();
  if let Some(current) = s.current
    && current == profile_name {
      exec_wg(&app_state, &current).await?;
      // Sleep for to let time for network to stabilize
      tokio::time::sleep(Duration::from_secs(1)).await;
      sync_connection_state(&app, &app_state).await?;
    };
  let path = format!("{}/profiles/{profile_name}.conf", s.conf_dir);
  let _ = fs::remove_file(path).await;
  Ok(())
}

#[tauri::command]
async fn connect_profile(
  app: AppHandle,
  app_state: State<'_, AppSt>,
  profile: String,
) -> Result<(), AppError> {
  let s = app_state.0.lock().await.clone();
  let conf_dir = s.conf_dir.clone();
  let current = s.current;
  if let Some(current) = current {
    exec_wg(&app_state, &current).await?;
  }
  exec_wg(&app_state, &profile).await?;
  tokio::fs::write(format!("{conf_dir}/current"), &profile.trim())
    .await
    .unwrap();
  // Sleep for 1 second to let time for network to stabilize
  tokio::time::sleep(Duration::from_secs(1)).await;
  sync_connection_state(&app, &app_state).await?;
  Ok(())
}

#[tauri::command]
async fn disconnect(
  app: AppHandle,
  app_state: State<'_, AppSt>,
) -> Result<(), AppError> {
  let s = app_state.0.lock().await.clone();
  let Some(current) = s.current else {
    return Ok(());
  };
  exec_wg(&app_state, &current).await?;
  let _ = fs::remove_file(format!("{}/current", s.conf_dir)).await;
  // Sleep for 1 second to let time for network to stabilize
  tokio::time::sleep(Duration::from_secs(1)).await;
  sync_connection_state(&app, &app_state).await?;
  Ok(())
}

#[tauri::command]
async fn update_profile(
  app: AppHandle,
  app_state: State<'_, AppSt>,
  profile_name: String,
  profile: ProfilePartial,
) -> Result<(), AppError> {
  let s = app_state.0.lock().await.clone();
  let path = format!("{}/profiles/{profile_name}.conf", s.conf_dir);
  if !fs::try_exists(&path).await.unwrap() {
    return Err(AppError {
      message: "Profile does not exists".into(),
    });
  }
  let mut is_current = false;
  if let Some(current) = s.current
    && profile_name == current {
      exec_wg(&app_state, &profile_name).await?;
      is_current = true;
    }
  fs::write(path, profile.content).await.unwrap();
  if !is_current {
    return Ok(());
  }
  exec_wg(&app_state, &profile_name).await?;
  tokio::time::sleep(Duration::from_secs(1)).await;
  sync_connection_state(&app, &app_state).await?;
  Ok(())
}

#[tauri::command]
async fn list_profile(
  app_state: State<'_, AppSt>,
) -> Result<Vec<Profile>, AppError> {
  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let path = format!("{conf_dir}/profiles");
  let mut dirs = fs::read_dir(path).await.unwrap();
  let mut profiles = Vec::new();
  while let Ok(Some(dir)) = dirs.next_entry().await {
    let path = dir.path();
    let file_name = path.file_name().unwrap().to_str().unwrap();
    if file_name.ends_with(".conf") {
      let content = fs::read_to_string(&path).await.unwrap_or_default();
      let profile = Profile {
        name: file_name.replace(".conf", "").to_string(),
        content,
      };
      profiles.push(profile);
    }
  }
  Ok(profiles)
}

#[tauri::command]
async fn import_profiles(
  app_state: State<'_, AppSt>,
  file_paths: Vec<String>,
) -> Result<ImportResult, AppError> {
  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let mut success = Vec::new();
  let mut failed = Vec::new();

  for file_path in file_paths {
    let path = std::path::Path::new(&file_path);

    // Get the file name
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
      failed.push(ImportError {
        file_name: file_path.clone(),
        error: "Invalid file name".to_string(),
      });
      continue;
    };

    // Check if it's a .conf file
    if !file_name.ends_with(".conf") {
      failed.push(ImportError {
        file_name: file_name.to_string(),
        error: "File must have .conf extension".to_string(),
      });
      continue;
    }

    // Read file content
    let content = match fs::read_to_string(path).await {
      Ok(c) => c,
      Err(e) => {
        failed.push(ImportError {
          file_name: file_name.to_string(),
          error: format!("Failed to read file: {}", e),
        });
        continue;
      }
    };

    // Validate content (minimum 8 characters)
    if content.len() < 8 {
      failed.push(ImportError {
        file_name: file_name.to_string(),
        error: "File content must be at least 8 characters".to_string(),
      });
      continue;
    }

    // Extract and sanitize profile name
    let base_name = file_name.replace(".conf", "");
    let sanitized_name: String = base_name
      .chars()
      .filter(|c| c.is_alphanumeric())
      .collect();

    if sanitized_name.is_empty() {
      failed.push(ImportError {
        file_name: file_name.to_string(),
        error: "Profile name must contain at least one alphanumeric character".to_string(),
      });
      continue;
    }

    // Handle duplicate names by appending a number
    let mut final_name = sanitized_name.clone();
    let mut counter = 1;
    loop {
      let target_path = format!("{}/profiles/{}.conf", conf_dir, final_name);
      if !fs::try_exists(&target_path).await.unwrap_or(false) {
        break;
      }
      final_name = format!("{}_{}", sanitized_name, counter);
      counter += 1;
    }

    // Write the profile
    let target_path = format!("{}/profiles/{}.conf", conf_dir, final_name);
    match fs::write(&target_path, content).await {
      Ok(_) => success.push(final_name),
      Err(e) => {
        failed.push(ImportError {
          file_name: file_name.to_string(),
          error: format!("Failed to write profile: {}", e),
        });
      }
    }
  }

  Ok(ImportResult { success, failed })
}

#[tauri::command]
async fn export_profiles(
  app_state: State<'_, AppSt>,
  target_dir: String,
) -> Result<ExportResult, AppError> {
  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let profiles_dir = format!("{}/profiles", conf_dir);

  let mut success = Vec::new();
  let mut failed = Vec::new();

  // Read all profiles from the profiles directory
  let mut dirs = match fs::read_dir(&profiles_dir).await {
    Ok(d) => d,
    Err(e) => {
      return Err(AppError {
        message: format!("Failed to read profiles directory: {}", e),
      });
    }
  };

  while let Ok(Some(entry)) = dirs.next_entry().await {
    let path = entry.path();
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
      Some(name) => name,
      None => continue,
    };

    // Only process .conf files
    if !file_name.ends_with(".conf") {
      continue;
    }

    let profile_name = file_name.replace(".conf", "");
    let target_path = format!("{}/{}", target_dir, file_name);

    // Copy the file to the target directory
    match fs::copy(&path, &target_path).await {
      Ok(_) => success.push(profile_name),
      Err(e) => {
        failed.push(ExportError {
          profile_name,
          error: format!("Failed to export: {}", e),
        });
      }
    }
  }

  Ok(ExportResult { success, failed })
}

fn build_tray(conn_st: &ConnSt, app: &App) -> Result<TrayIcon, Box<dyn std::error::Error>> {
  let title = MenuItem::with_id(app, "title", APP_TITLE, false, None::<&str>)?;
  let open_i = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
  let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
  let menu = Menu::with_items(app, &[&title, &open_i, &quit_i])?;
  let image = if *conn_st == ConnSt::Connected {
    Image::from_bytes(TRAY_CONNECTED_ICON)?
  } else {
    Image::from_bytes(TRAY_DISCONNECTED_ICON)?
  };
  let tray = TrayIconBuilder::with_id("main")
      .on_menu_event(move |app, event| {
        match event.id.as_ref() {
          "quit" => {
            app.exit(0);
          }
          "open" => {
            if let Some(window) = app.get_webview_window("main") {
              let _ = window.center();
              let _ = window.set_focus();
              let _ = window.show();
            }
          }
          _ => {}
        }
    })
  .icon(image)
  .menu(&menu)
  .tooltip(APP_TITLE)
  .icon_as_template(true)
  .show_menu_on_left_click(true)
  .build(app)?;
  Ok(tray)
}

#[cfg(target_os = "linux")]
fn setup_appimage_gl_workarounds() {
  use std::env;

  fn set_env_if_unset(key: &str, value: &str) {
    if env::var_os(key).is_none() {
      // SAFETY: this runs during process startup before the app initializes
      // WebKit/Tauri state that depends on these variables.
      unsafe {
        env::set_var(key, value);
      }
    }
  }

  if env::var_os("APPIMAGE").is_some() {
    // Prefer software GL to avoid EGL surfaceless failures
    set_env_if_unset("LIBGL_ALWAYS_SOFTWARE", "1");
    set_env_if_unset("MESA_LOADER_DRIVER_OVERRIDE", "llvmpipe");
    // Avoid Wayland/DMABUF renderer path that often triggers EGL_BAD_ALLOC
    set_env_if_unset("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    // As a fallback, disable accelerated compositing
    set_env_if_unset("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    // Optional: force X11 if Wayland causes issues (comment out if not needed)
    // if env::var_os("GDK_BACKEND").is_none() { env::set_var("GDK_BACKEND", "x11"); }
  }
}

#[tokio::main]
async fn main() {
  #[cfg(target_os = "linux")]
  setup_appimage_gl_workarounds();
  let app_state = init_app_st().await;
  let conn_st = app_state.0.lock().await.conn_st.clone();
  let managed_app_state = app_state.clone();
  let setup_app_state = app_state.clone();
  // let system_tray = create_tray_menu(&app_state).await;
  tauri::Builder::default()
  .on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
      let _ = window.set_always_on_top(false);
      let _ = window.hide();
      api.prevent_close();
    }
  })
  .setup(move |app| {
    build_tray(&conn_st, app)?;
    let app_handle = app.handle().clone();
    let monitor_state = setup_app_state.clone();
    tauri::async_runtime::spawn(async move {
      let _ = sync_connection_state(&app_handle, &monitor_state).await;
      let mut interval = tokio::time::interval(Duration::from_secs(1));
      interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
      loop {
        interval.tick().await;
        let _ = sync_connection_state(&app_handle, &monitor_state).await;
      }
    });
    Ok(())
  })
    .manage(managed_app_state)
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      get_state,
      list_profile,
      connect_profile,
      disconnect,
      create_profile,
      delete_profile,
      update_profile,
      import_profiles,
      export_profiles,
    ])
    // .plugin(tauri_plugin_window_state::Builder::default().build())
    .build(tauri::generate_context!())
    .expect("error while running tauri application")
    .run(|_, _| {});
}
