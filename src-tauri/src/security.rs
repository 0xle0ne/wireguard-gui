use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::{
  aead::{Aead, KeyInit},
  Key, XChaCha20Poly1305, XNonce,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

const KEY_CHECK_PLAINTEXT: &str = "wireguard-gui-key-check-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
  pub encryption_enabled: bool,
  pub salt_b64: String,
  pub verifier: String,
}

impl Default for SecurityConfig {
  fn default() -> Self {
    Self {
      encryption_enabled: false,
      salt_b64: String::new(),
      verifier: String::new(),
    }
  }
}

pub fn validate_pin(pin: &str) -> bool {
  pin.len() == 4 && pin.chars().all(|c| c.is_ascii_digit())
}

pub fn derive_key_from_pin(pin: &str, salt: &[u8]) -> Result<[u8; 32], String> {
  let mut key = [0u8; 32];
  Argon2::default()
    .hash_password_into(pin.as_bytes(), salt, &mut key)
    .map_err(|err| format!("Failed to derive key: {err}"))?;
  Ok(key)
}

pub fn generate_salt() -> [u8; 16] {
  let mut salt = [0u8; 16];
  rand::rng().fill(&mut salt);
  salt
}

pub fn encrypt_value(plaintext: &str, key: &[u8; 32]) -> Result<String, String> {
  let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
  let mut nonce_buf = [0u8; 24];
  rand::rng().fill(&mut nonce_buf);
  let nonce = XNonce::from_slice(&nonce_buf);
  let encrypted = cipher
    .encrypt(nonce, plaintext.as_bytes())
    .map_err(|err| format!("Encryption failed: {err}"))?;
  Ok(format!("{}:{}", B64.encode(nonce_buf), B64.encode(encrypted)))
}

pub fn decrypt_value(payload: &str, key: &[u8; 32]) -> Result<String, String> {
  let Some((nonce_b64, data_b64)) = payload.split_once(':') else {
    return Err("Invalid encrypted payload format".to_string());
  };
  let nonce_bytes = B64
    .decode(nonce_b64)
    .map_err(|err| format!("Invalid nonce encoding: {err}"))?;
  if nonce_bytes.len() != 24 {
    return Err("Invalid nonce length".to_string());
  }
  let data = B64
    .decode(data_b64)
    .map_err(|err| format!("Invalid data encoding: {err}"))?;
  let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
  let nonce = XNonce::from_slice(&nonce_bytes);
  let decrypted = cipher
    .decrypt(nonce, data.as_ref())
    .map_err(|_| "Failed to decrypt payload".to_string())?;
  String::from_utf8(decrypted).map_err(|err| format!("Invalid UTF-8 content: {err}"))
}

pub fn build_security_config(pin: &str) -> Result<SecurityConfig, String> {
  let salt = generate_salt();
  let key = derive_key_from_pin(pin, &salt)?;
  let verifier = encrypt_value(KEY_CHECK_PLAINTEXT, &key)?;
  Ok(SecurityConfig {
    encryption_enabled: true,
    salt_b64: B64.encode(salt),
    verifier,
  })
}

pub fn verify_pin(config: &SecurityConfig, pin: &str) -> Result<[u8; 32], String> {
  let salt = B64
    .decode(&config.salt_b64)
    .map_err(|err| format!("Invalid security salt: {err}"))?;
  let key = derive_key_from_pin(pin, &salt)?;
  let check = decrypt_value(&config.verifier, &key)?;
  if check != KEY_CHECK_PLAINTEXT {
    return Err("Invalid PIN".to_string());
  }
  Ok(key)
}
