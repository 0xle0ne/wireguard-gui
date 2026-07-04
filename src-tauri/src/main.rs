#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, os::unix::fs::PermissionsExt, sync::Arc};

use tokio::{
  fs,
  process::Command,
  sync::Mutex,
  time::{timeout, Duration, MissedTickBehavior},
};
use serde::{Serialize, Deserialize};

use tauri::{
  image::Image, menu::{Menu, MenuItem}, tray::{TrayIcon, TrayIconBuilder}, App, AppHandle, Emitter, Manager, State
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

mod security;
use security::{
  build_security_config, decrypt_value, encrypt_value, validate_pin, verify_pin,
  SecurityConfig,
};

const WG_SCRIPT: &str = include_str!("../scripts/wg.sh");

const WG_ZENITY_SCRIPT: &str = include_str!("../scripts/zenity.sh");

const APP_TITLE: &str = "Wireguard GUI";
const APP_STATE_CHANGED_EVENT: &str = "app-state-changed";
const NM_CONN_PREFIX: &str = "wg-gui-";
const PROFILE_EXT_PLAIN: &str = ".conf";
const PROFILE_EXT_ENCRYPTED: &str = ".conf.enc";
const SECURITY_CONFIG_FILE: &str = "security.json";

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
  pub encryption_enabled: bool,
  pub is_unlocked: bool,
  #[serde(skip_serializing, skip_deserializing)]
  pub unlock_key: Option<[u8; 32]>,
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
      encryption_enabled: false,
      is_unlocked: false,
      unlock_key: None,
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
  code: Option<String>,
}

impl AppError {
  fn msg(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      code: None,
    }
  }

  fn coded(code: &str, message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      code: Some(code.to_owned()),
    }
  }
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

fn security_config_path(conf_dir: &str) -> String {
  format!("{conf_dir}/{SECURITY_CONFIG_FILE}")
}

fn profile_plain_path(conf_dir: &str, name: &str) -> String {
  format!("{conf_dir}/profiles/{name}{PROFILE_EXT_PLAIN}")
}

fn profile_enc_path(conf_dir: &str, name: &str) -> String {
  format!("{conf_dir}/profiles/{name}{PROFILE_EXT_ENCRYPTED}")
}

async fn load_security_config(conf_dir: &str) -> SecurityConfig {
  let path = security_config_path(conf_dir);
  let Ok(raw) = fs::read_to_string(path).await else {
    return SecurityConfig::default();
  };
  serde_json::from_str::<SecurityConfig>(&raw).unwrap_or_default()
}

async fn save_security_config(conf_dir: &str, config: &SecurityConfig) -> Result<(), AppError> {
  let path = security_config_path(conf_dir);
  let payload = serde_json::to_string_pretty(config)
    .map_err(|e| AppError::coded("security_config_invalid", format!("Invalid security config: {e}")))?;
  fs::write(path, payload)
    .await
    .map_err(|e| AppError::coded("security_config_write_failed", format!("Failed to save security config: {e}")))
}

async fn ensure_unlocked_if_needed(app_state: &AppSt) -> Result<Option<[u8; 32]>, AppError> {
  let s = app_state.0.lock().await;
  if !s.encryption_enabled {
    return Ok(None);
  }
  if !s.is_unlocked {
    return Err(AppError::coded(
      "profiles_locked",
      "Profiles are encrypted and the app is locked",
    ));
  }
  let Some(key) = s.unlock_key else {
    return Err(AppError::coded("profiles_locked", "Missing unlock key"));
  };
  Ok(Some(key))
}

async fn profile_exists(conf_dir: &str, name: &str, encrypted: bool) -> bool {
  let path = if encrypted {
    profile_enc_path(conf_dir, name)
  } else {
    profile_plain_path(conf_dir, name)
  };
  fs::try_exists(path).await.unwrap_or(false)
}

async fn read_profile_content(app_state: &AppSt, profile_name: &str) -> Result<String, AppError> {
  let (conf_dir, encrypted, key_opt) = {
    let s = app_state.0.lock().await;
    (s.conf_dir.clone(), s.encryption_enabled, s.unlock_key)
  };

  if encrypted {
    let Some(key) = key_opt else {
      return Err(AppError::coded("profiles_locked", "Profiles are encrypted and locked"));
    };
    let raw = fs::read_to_string(profile_enc_path(&conf_dir, profile_name))
      .await
      .map_err(|e| AppError::coded("profile_read_failed", format!("Failed to read encrypted profile: {e}")))?;
    return decrypt_value(&raw, &key)
      .map_err(|e| AppError::coded("profile_decrypt_failed", format!("Failed to decrypt profile: {e}")));
  }

  fs::read_to_string(profile_plain_path(&conf_dir, profile_name))
    .await
    .map_err(|e| AppError::coded("profile_read_failed", format!("Failed to read profile: {e}")))
}

async fn write_profile_content(
  app_state: &AppSt,
  profile_name: &str,
  content: &str,
) -> Result<(), AppError> {
  let (conf_dir, encrypted, key_opt) = {
    let s = app_state.0.lock().await;
    (s.conf_dir.clone(), s.encryption_enabled, s.unlock_key)
  };

  if encrypted {
    let Some(key) = key_opt else {
      return Err(AppError::coded("profiles_locked", "Profiles are encrypted and locked"));
    };
    let encrypted_payload = encrypt_value(content, &key)
      .map_err(|e| AppError::coded("profile_encrypt_failed", format!("Failed to encrypt profile: {e}")))?;
    fs::write(profile_enc_path(&conf_dir, profile_name), encrypted_payload)
      .await
      .map_err(|e| AppError::coded("profile_write_failed", format!("Failed to write encrypted profile: {e}")))?;
    return Ok(());
  }

  fs::write(profile_plain_path(&conf_dir, profile_name), content)
    .await
    .map_err(|e| AppError::coded("profile_write_failed", format!("Failed to write profile: {e}")))
}

async fn delete_profile_content(app_state: &AppSt, profile_name: &str) -> Result<(), AppError> {
  let (conf_dir, encrypted) = {
    let s = app_state.0.lock().await;
    (s.conf_dir.clone(), s.encryption_enabled)
  };
  let path = if encrypted {
    profile_enc_path(&conf_dir, profile_name)
  } else {
    profile_plain_path(&conf_dir, profile_name)
  };
  let _ = fs::remove_file(path).await;
  Ok(())
}

async fn list_profile_names(conf_dir: &str, encrypted: bool) -> Result<Vec<String>, AppError> {
  let mut dirs = fs::read_dir(format!("{conf_dir}/profiles"))
    .await
    .map_err(|e| AppError::coded("profile_dir_read_failed", format!("Failed to read profiles directory: {e}")))?;
  let ext = if encrypted {
    PROFILE_EXT_ENCRYPTED
  } else {
    PROFILE_EXT_PLAIN
  };
  let mut names = Vec::new();
  while let Ok(Some(dir)) = dirs.next_entry().await {
    let path = dir.path();
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    if file_name.ends_with(ext) {
      names.push(file_name.trim_end_matches(ext).to_string());
    }
  }
  Ok(names)
}

async fn migrate_plain_to_encrypted(app_state: &AppSt, key: &[u8; 32]) -> Result<(), AppError> {
  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let names = list_profile_names(&conf_dir, false).await?;
  for name in names {
    let raw = fs::read_to_string(profile_plain_path(&conf_dir, &name))
      .await
      .map_err(|e| AppError::coded("profile_read_failed", format!("Failed to read profile: {e}")))?;
    let encrypted = encrypt_value(&raw, key)
      .map_err(|e| AppError::coded("profile_encrypt_failed", format!("Failed to encrypt profile: {e}")))?;
    fs::write(profile_enc_path(&conf_dir, &name), encrypted)
      .await
      .map_err(|e| AppError::coded("profile_write_failed", format!("Failed to write profile: {e}")))?;
    let _ = fs::remove_file(profile_plain_path(&conf_dir, &name)).await;
  }
  Ok(())
}

async fn migrate_encrypted_to_plain(app_state: &AppSt, key: &[u8; 32]) -> Result<(), AppError> {
  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let names = list_profile_names(&conf_dir, true).await?;
  for name in names {
    let raw = fs::read_to_string(profile_enc_path(&conf_dir, &name))
      .await
      .map_err(|e| AppError::coded("profile_read_failed", format!("Failed to read encrypted profile: {e}")))?;
    let plain = decrypt_value(&raw, key)
      .map_err(|e| AppError::coded("profile_decrypt_failed", format!("Failed to decrypt profile: {e}")))?;
    fs::write(profile_plain_path(&conf_dir, &name), plain)
      .await
      .map_err(|e| AppError::coded("profile_write_failed", format!("Failed to write profile: {e}")))?;
    let _ = fs::remove_file(profile_enc_path(&conf_dir, &name)).await;
  }
  Ok(())
}

async fn prepare_plain_profile_for_exec(app_state: &AppSt, profile_name: &str) -> Result<Option<String>, AppError> {
  let (conf_dir, encrypted) = {
    let s = app_state.0.lock().await;
    (s.conf_dir.clone(), s.encryption_enabled)
  };

  if !encrypted {
    return Ok(None);
  }

  let content = read_profile_content(app_state, profile_name).await?;
  let plain_path = profile_plain_path(&conf_dir, profile_name);
  fs::write(&plain_path, content)
    .await
    .map_err(|e| AppError::coded("profile_write_failed", format!("Failed to prepare decrypted profile: {e}")))?;
  Ok(Some(plain_path))
}

fn profile_from_nm_conn_name(name: &str) -> Option<String> {
  name.strip_prefix(NM_CONN_PREFIX).map(str::to_owned)
}

fn is_snap_mode() -> bool {
  std::env::var_os("IS_SNAP").is_some()
}

fn is_valid_profile_name(name: &str) -> bool {
  !name.is_empty()
    && name.len() <= 15
    && name
      .chars()
      .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '=' || c == '-')
}

async fn is_nmcli_available() -> bool {
  match timeout(
    Duration::from_secs(2),
    Command::new("nmcli").args(["--version"]).output(),
  )
  .await
  {
    Ok(Ok(output)) => output.status.success(),
    _ => false,
  }
}

async fn get_active_nm_profile(current_hint: Option<&str>, conf_dir: &str) -> Option<String> {
  let output = match timeout(
    Duration::from_secs(3),
    Command::new("nmcli")
      .args(["-t", "-f", "NAME,TYPE", "connection", "show", "--active"])
      .output(),
  )
  .await
  {
    Ok(Ok(out)) => out,
    Ok(Err(_)) => return None,
    Err(_) => return None,
  };

  if !output.status.success() {
    return None;
  }

  let stdout_str = String::from_utf8_lossy(&output.stdout);

  let active_wireguard_names = stdout_str
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

  if let Some(current) = current_hint {
    let prefixed = format!("{NM_CONN_PREFIX}{current}");
    if active_wireguard_names.iter().any(|name| name == current || name == &prefixed) {
      return Some(current.to_owned());
    }
    return None;
  }

  let mut detected = None;
  for name in &active_wireguard_names {
    // Only auto-detect app-managed connection names.
    let Some(candidate) = profile_from_nm_conn_name(name) else {
      continue;
    };
    let profile_plain = profile_plain_path(conf_dir, &candidate);
    let profile_enc = profile_enc_path(conf_dir, &candidate);
    if fs::try_exists(&profile_plain).await.unwrap_or(false)
      || fs::try_exists(&profile_enc).await.unwrap_or(false) {
      detected = Some(candidate);
      break;
    }
  }
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
  let is_snap = is_snap_mode();
  let has_nmcli = is_nmcli_available().await;
  if is_snap {
    let snap_hint = match current_hint {
      Some(current) => Some(current),
      None => read_current_profile(conf_dir).await,
    };

    if has_nmcli
      && let Some(profile) = get_active_nm_profile(snap_hint.as_deref(), conf_dir).await {
      return (ConnSt::Connected, Some(profile));
    }

    return (ConnSt::Disconnected, None);
  }

  let current = match current_hint {
    Some(current) => Some(current),
    None => read_current_profile(conf_dir).await,
  };

  if let Some(current_name) = current.as_deref()
    && get_con_st(current_name).await == ConnSt::Connected {
      return (ConnSt::Connected, Some(current_name.to_owned()));
  }

  // Fallback: check NM for active WireGuard connections (handles both
  // wg-gui-<name> prefixed and exact-name connections) - only if available
  if has_nmcli
    && let Some(profile) = get_active_nm_profile(current.as_deref(), conf_dir).await {
    return (ConnSt::Connected, Some(profile));
  }

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
  if !state_changed && next_pub_ip == prev_pub_ip {
    return Ok(());
  }

  {
    let mut s = app_state.0.lock().await;
    s.conn_st = next_conn_st.clone();
    s.current = next_current.clone();
    s.pub_ip = next_pub_ip;
  }

  if next_conn_st == ConnSt::Connected {
    if let Some(current) = next_current {
      let _ = fs::write(format!("{conf_dir}/current"), current.trim()).await;
    }
  } else {
    let _ = fs::remove_file(format!("{conf_dir}/current")).await;
  }

  if let Some(tray) = app.tray_by_id("main") {
    let icon = if next_conn_st == ConnSt::Connected {
      TRAY_CONNECTED_ICON
    } else {
      TRAY_DISCONNECTED_ICON
    };
    let _ = tray.set_icon(Some(Image::from_bytes(icon).unwrap()));
  }

  let payload = app_state.0.lock().await.clone();
  let _ = app.emit(APP_STATE_CHANGED_EVENT, payload);
  Ok(())
}

async fn get_con_st(current: &str) -> ConnSt {
  let output = Command::new("ip")
    .args(["-br", "link", "show", "dev", current])
    .output()
    .await
    .expect("ip command failed");
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
  let security_config = load_security_config(&conf_dir).await;
  let app_state = AppSt(Arc::new(Mutex::new(default_state)));
  create_scripts(&conf_dir).await;
  let (conn_st, current) = detect_connection_state(current, &conf_dir).await;
  let mut s = app_state.0.lock().await;
  s.pub_ip = (get_pub_ip().await).ok();
  s.conn_st = conn_st;
  s.current = current;
  s.encryption_enabled = security_config.encryption_enabled;
  s.is_unlocked = !security_config.encryption_enabled;
  s.unlock_key = None;
  if s.current.is_none() {
    let _ = fs::remove_file(format!("{}/current", s.conf_dir)).await;
  }
  app_state.clone()
}

async fn exec_wg(app_state: &AppSt, profile: &str) -> Result<(), AppError> {
  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let mut envs = HashMap::new();
  envs.insert("PROFILE".to_owned(), profile);

  let is_snap = is_snap_mode();
  if is_snap {
    envs.insert("IS_SNAP".to_owned(), "true");
    println!("[wg-gui] exec_wg: running in snap environment for profile {}", profile);
  } else {
    println!("[wg-gui] exec_wg: running in native environment for profile {}", profile);
  }

  println!("[wg-gui] exec_wg: executing wg.sh for profile {}", profile);
  let res = timeout(
    Duration::from_secs(20),
    Command::new("bash")
      .args([format!("{conf_dir}/wg.sh")])
      .envs(envs)
      .output(),
  )
  .await
  .map_err(|_| AppError::coded("timeout", "wg operation timed out"))?
  .map_err(|e| AppError::coded("script_exec_failed", format!("Failed to execute wg.sh: {}", e)))?;

  println!(
    "[wg-gui] exec_wg: wg.sh exit code: {:?}",
    res.status.code()
  );
  println!(
    "[wg-gui] exec_wg: wg.sh stdout: {}",
    String::from_utf8_lossy(&res.stdout)
  );
  println!(
    "[wg-gui] exec_wg: wg.sh stderr: {}",
    String::from_utf8_lossy(&res.stderr)
  );

  if res.status.code().unwrap_or_default() != 0 {
    let error_msg = String::from_utf8(res.stderr).unwrap_or_default().trim().to_owned();
    let exit_code = res.status.code().unwrap_or(-1);
    let lower = error_msg.to_ascii_lowercase();

    let coded_error = if exit_code == 127 || lower.contains("nmcli") && lower.contains("required") {
      AppError::coded("nmcli_missing", if error_msg.is_empty() { "nmcli is required but unavailable" } else { error_msg.as_str() })
    } else if exit_code == 124 {
      AppError::coded("timeout", if error_msg.is_empty() { "wg operation timed out" } else { error_msg.as_str() })
    } else if lower.contains("permission denied") || lower.contains("not authorized") {
      AppError::coded("permission_denied", if error_msg.is_empty() { "Permission denied while managing network connection" } else { error_msg.as_str() })
    } else if lower.contains("failed to import") {
      AppError::coded("import_failed", if error_msg.is_empty() { "Failed to import WireGuard profile" } else { error_msg.as_str() })
    } else if lower.contains("failed to bring up") || lower.contains("activation") {
      AppError::coded("activation_failed", if error_msg.is_empty() { "Failed to activate WireGuard connection" } else { error_msg.as_str() })
    } else {
      AppError::coded(
        "script_failed",
        if error_msg.is_empty() {
          format!("wg.sh failed with exit code {}", exit_code)
        } else {
          error_msg
        },
      )
    };

    return Err(coded_error);
  }

  println!("[wg-gui] exec_wg: successfully executed wg.sh for profile {}", profile);
  Ok(())
}

async fn exec_wg_profile(app_state: &AppSt, profile: &str) -> Result<(), AppError> {
  let temp_plain = prepare_plain_profile_for_exec(app_state, profile).await?;
  let result = exec_wg(app_state, profile).await;
  if let Some(path) = temp_plain {
    let _ = fs::remove_file(path).await;
  }
  result
}

async fn get_pub_ip() -> Result<String, AppError> {
  let payload = reqwest::get("https://httpbin.org/ip")
    .await
    .map_err(|err| AppError::msg(err.to_string()))?
    .json::<IpPayload>()
    .await.map_err(|err| AppError::msg(err.to_string()))?;
  Ok(payload.origin)
}

#[tauri::command]
async fn get_state(
  app_state: State<'_, AppSt>,
) -> Result<AppStInner, AppError> {
  Ok(app_state.0.lock().await.clone())
}

#[tauri::command]
async fn enable_profile_encryption(
  app_state: State<'_, AppSt>,
  pin: String,
) -> Result<(), AppError> {
  if !validate_pin(&pin) {
    return Err(AppError::coded("invalid_pin", "PIN must be exactly 4 digits"));
  }

  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let mut config = load_security_config(&conf_dir).await;
  if config.encryption_enabled {
    return Err(AppError::coded(
      "encryption_already_enabled",
      "Profile encryption is already enabled",
    ));
  }

  config = build_security_config(&pin)
    .map_err(|e| AppError::coded("security_setup_failed", format!("Failed to initialize encryption: {e}")))?;
  let key = verify_pin(&config, &pin)
    .map_err(|e| AppError::coded("invalid_pin", e))?;

  {
    let mut s = app_state.0.lock().await;
    s.encryption_enabled = true;
    s.is_unlocked = true;
    s.unlock_key = Some(key);
  }

  migrate_plain_to_encrypted(&app_state, &key).await?;
  save_security_config(&conf_dir, &config).await?;
  Ok(())
}

#[tauri::command]
async fn disable_profile_encryption(
  app_state: State<'_, AppSt>,
  pin: String,
) -> Result<(), AppError> {
  if !validate_pin(&pin) {
    return Err(AppError::coded("invalid_pin", "PIN must be exactly 4 digits"));
  }

  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let config = load_security_config(&conf_dir).await;
  if !config.encryption_enabled {
    return Err(AppError::coded(
      "encryption_not_enabled",
      "Profile encryption is not enabled",
    ));
  }

  let key = verify_pin(&config, &pin)
    .map_err(|_| AppError::coded("pin_incorrect", "Incorrect PIN"))?;

  {
    let mut s = app_state.0.lock().await;
    s.encryption_enabled = true;
    s.is_unlocked = true;
    s.unlock_key = Some(key);
  }

  migrate_encrypted_to_plain(&app_state, &key).await?;
  save_security_config(&conf_dir, &SecurityConfig::default()).await?;

  {
    let mut s = app_state.0.lock().await;
    s.encryption_enabled = false;
    s.is_unlocked = true;
    s.unlock_key = None;
  }

  Ok(())
}

#[tauri::command]
async fn unlock_profiles(app_state: State<'_, AppSt>, pin: String) -> Result<(), AppError> {
  if !validate_pin(&pin) {
    return Err(AppError::coded("invalid_pin", "PIN must be exactly 4 digits"));
  }
  let conf_dir = app_state.0.lock().await.conf_dir.clone();
  let config = load_security_config(&conf_dir).await;
  if !config.encryption_enabled {
    return Err(AppError::coded(
      "encryption_not_enabled",
      "Profile encryption is not enabled",
    ));
  }

  let key = verify_pin(&config, &pin)
    .map_err(|_| AppError::coded("pin_incorrect", "Incorrect PIN"))?;

  let mut s = app_state.0.lock().await;
  s.is_unlocked = true;
  s.unlock_key = Some(key);
  Ok(())
}

#[tauri::command]
async fn lock_profiles(app_state: State<'_, AppSt>) -> Result<(), AppError> {
  let mut s = app_state.0.lock().await;
  if !s.encryption_enabled {
    return Ok(());
  }
  s.is_unlocked = false;
  s.unlock_key = None;
  Ok(())
}

#[tauri::command]
async fn reset_app_data(app_state: State<'_, AppSt>) -> Result<(), AppError> {
  let (conf_dir, current) = {
    let s = app_state.0.lock().await;
    (s.conf_dir.clone(), s.current.clone())
  };

  if let Some(profile_name) = current {
    let _ = exec_wg_profile(&app_state, &profile_name).await;
  }

  let _ = fs::remove_dir_all(&conf_dir).await;
  create_scripts(&conf_dir).await;

  {
    let mut s = app_state.0.lock().await;
    s.current = None;
    s.pub_ip = None;
    s.conn_st = ConnSt::Disconnected;
    s.encryption_enabled = false;
    s.is_unlocked = true;
    s.unlock_key = None;
  }

  Ok(())
}

#[tauri::command]
async fn create_profile(
  app_state: State<'_, AppSt>,
  new_profile: ProfilePartial,
) -> Result<(), AppError> {
  let s = app_state.0.lock().await.clone();
  // Accept Linux interface-compatible profile names.
  let name = new_profile.name;
  if !is_valid_profile_name(&name) {
      return Err(AppError::coded(
        "invalid_profile_name",
        "Name must be 1-15 chars and contain only alphanumeric values, _, -, ., or =",
      ));
  }
  if s.encryption_enabled {
    let _ = ensure_unlocked_if_needed(&app_state).await?;
  }
  if profile_exists(&s.conf_dir, &name, s.encryption_enabled).await {
    return Err(AppError::coded("profile_exists", "Profile already exists"));
  }
  write_profile_content(&app_state, &name, &new_profile.content).await?;
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
      exec_wg_profile(&app_state, &current).await?;
      // Sleep for to let time for network to stabilize
      tokio::time::sleep(Duration::from_secs(1)).await;
      sync_connection_state(&app, &app_state).await?;
    };
  delete_profile_content(&app_state, &profile_name).await?;
  Ok(())
}

#[tauri::command]
async fn connect_profile(
  app: AppHandle,
  app_state: State<'_, AppSt>,
  profile: String,
) -> Result<(), AppError> {
  let _ = ensure_unlocked_if_needed(&app_state).await?;
  let s = app_state.0.lock().await.clone();
  let conf_dir = s.conf_dir.clone();
  let current = s.current;
  if let Some(current) = current {
    exec_wg_profile(&app_state, &current).await?;
  }
  exec_wg_profile(&app_state, &profile).await?;
  tokio::fs::write(format!("{conf_dir}/current"), &profile.trim())
    .await
    .map_err(|e| AppError::coded("state_write_failed", format!("Failed to persist current profile: {}", e)))?;
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
  exec_wg_profile(&app_state, &current).await?;
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
  if s.encryption_enabled {
    let _ = ensure_unlocked_if_needed(&app_state).await?;
  }
  if !profile_exists(&s.conf_dir, &profile_name, s.encryption_enabled).await {
    return Err(AppError::coded("profile_not_found", "Profile does not exist"));
  }

  let mut is_current = false;
  if let Some(current) = s.current.as_ref()
    && profile_name == *current {
      println!(
        "[wg-gui] update_profile: profile {} is currently active, will reconnect after update",
        profile_name
      );
      // Disconnect the active profile
      println!("[wg-gui] update_profile: disconnecting {}", profile_name);
      if let Err(e) = exec_wg_profile(&app_state, &profile_name).await {
        println!("[wg-gui] update_profile: error disconnecting: {}", e.message);
        return Err(e);
      }
      is_current = true;
    }

  // Write the new profile content
  println!("[wg-gui] update_profile: writing new profile content for {}", profile_name);
  write_profile_content(&app_state, &profile_name, &profile.content).await?;

  if !is_current {
    println!("[wg-gui] update_profile: profile is not active, no reconnection needed");
    return Ok(());
  }

  // Reconnect with the updated profile
  println!("[wg-gui] update_profile: reconnecting with updated profile {}", profile_name);
  exec_wg_profile(&app_state, &profile_name).await?;

  // Wait for network to stabilize
  tokio::time::sleep(Duration::from_secs(1)).await;

  // Sync state
  println!("[wg-gui] update_profile: syncing connection state");
  sync_connection_state(&app, &app_state).await?;

  println!("[wg-gui] update_profile: successfully updated and reconnected {}", profile_name);
  Ok(())
}

#[tauri::command]
async fn list_profile(
  app_state: State<'_, AppSt>,
) -> Result<Vec<Profile>, AppError> {
  let s = app_state.0.lock().await.clone();
  if s.encryption_enabled {
    let _ = ensure_unlocked_if_needed(&app_state).await?;
  }
  let names = list_profile_names(&s.conf_dir, s.encryption_enabled).await?;
  let mut profiles = Vec::new();
  for name in names {
    let content = read_profile_content(&app_state, &name).await?;
    profiles.push(Profile { name, content });
  }
  Ok(profiles)
}

#[tauri::command]
async fn import_profiles(
  app_state: State<'_, AppSt>,
  file_paths: Vec<String>,
) -> Result<ImportResult, AppError> {
  let s = app_state.0.lock().await.clone();
  if s.encryption_enabled {
    let _ = ensure_unlocked_if_needed(&app_state).await?;
  }
  let conf_dir = s.conf_dir.clone();
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
    let mut sanitized_name: String = base_name
      .chars()
      .map(|c| {
        if c.is_alphanumeric() || c == '_' || c == '.' || c == '=' || c == '-' {
          c
        } else {
          '_'
        }
      })
      .collect();

    if sanitized_name.len() > 15 {
      sanitized_name.truncate(15);
    }

    if sanitized_name.is_empty() {
      failed.push(ImportError {
        file_name: file_name.to_string(),
        error: "Profile name must contain valid interface characters".to_string(),
      });
      continue;
    }

    // Handle duplicate names by appending a number while respecting 15-char limit.
    let mut final_name = sanitized_name.clone();
    let mut counter = 1;
    loop {
      if !profile_exists(&conf_dir, &final_name, s.encryption_enabled).await {
        break;
      }
      let suffix = format!("_{}", counter);
      let keep_len = 15usize.saturating_sub(suffix.len());
      if keep_len == 0 {
        failed.push(ImportError {
          file_name: file_name.to_string(),
          error: "Could not create a unique profile name within 15 characters".to_string(),
        });
        final_name.clear();
        break;
      }
      let base_prefix: String = sanitized_name.chars().take(keep_len).collect();
      final_name = format!("{}{}", base_prefix, suffix);
      counter += 1;
    }

    if final_name.is_empty() || !is_valid_profile_name(&final_name) {
      continue;
    }

    // Write the profile
    match write_profile_content(&app_state, &final_name, &content).await {
      Ok(_) => success.push(final_name),
      Err(e) => {
        failed.push(ImportError {
          file_name: file_name.to_string(),
          error: e.message,
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
  let s = app_state.0.lock().await.clone();
  if s.encryption_enabled {
    let _ = ensure_unlocked_if_needed(&app_state).await?;
  }
  let names = list_profile_names(&s.conf_dir, s.encryption_enabled).await?;

  let mut success = Vec::new();
  let mut failed = Vec::new();

  for profile_name in names {
    let target_path = format!("{}/{}{}", target_dir, profile_name, PROFILE_EXT_PLAIN);
    match read_profile_content(&app_state, &profile_name).await {
      Ok(content) => match fs::write(&target_path, content).await {
        Ok(_) => success.push(profile_name),
        Err(e) => failed.push(ExportError {
          profile_name,
          error: format!("Failed to export: {}", e),
        }),
      },
      Err(e) => failed.push(ExportError {
        profile_name,
        error: e.message,
      }),
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
            // will save the state of all open windows to disk
            let _ = app.save_window_state(StateFlags::all());
            app.exit(0);
          }
          "open" => {
            if let Some(window) = app.get_webview_window("main") {
              let _ = window.restore_state(StateFlags::all());
              let _ = window.show();
              let _ = window.set_focus();
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
    .plugin(tauri_plugin_wdio::init())
    .plugin(tauri_plugin_wdio_webdriver::init())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      if let Some(window) = app.get_webview_window("main") {
        let _ = window.restore_state(StateFlags::all());
        let _ = window.show();
        let _ = window.set_focus();
      }
    }))
    .invoke_handler(tauri::generate_handler![
      get_state,
      enable_profile_encryption,
      disable_profile_encryption,
      unlock_profiles,
      lock_profiles,
      reset_app_data,
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
