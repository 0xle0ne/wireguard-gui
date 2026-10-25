//! Private, atomic profile writes: never truncate the only copy of a private key.
use std::{
  fs::{self, OpenOptions},
  io::{self, Write},
  os::unix::fs::OpenOptionsExt,
  path::Path,
};

pub fn write_private(path: &str, content: &[u8]) -> io::Result<()> {
  let destination = Path::new(path);
  let parent = destination
    .parent()
    .ok_or_else(|| io::Error::other("Missing parent directory"))?;
  let temporary = parent.join(format!(
    ".wg-write-{}-{}",
    std::process::id(),
    rand::random::<u64>()
  ));
  let result = (|| {
    let mut file = OpenOptions::new()
      .write(true)
      .create_new(true)
      .mode(0o600)
      .open(&temporary)?;
    file.write_all(content)?;
    file.sync_all()?;
    fs::rename(&temporary, destination)?;
    // A failed directory sync cannot undo rename. Report the committed write
    // as successful so callers never roll back data referenced by this file.
    let _ = fs::File::open(parent).and_then(|directory| directory.sync_all());
    Ok(())
  })();
  if result.is_err() {
    let _ = fs::remove_file(&temporary);
  }
  result
}

// Export must never overwrite an unrelated file or follow an existing symlink.
pub fn export_private(path: &str, content: &[u8]) -> io::Result<()> {
  let mut file = OpenOptions::new()
    .write(true)
    .create_new(true)
    .mode(0o600)
    .open(path)?;
  if let Err(error) = file.write_all(content).and_then(|_| file.sync_all()) {
    let _ = fs::remove_file(path);
    return Err(error);
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::{symlink, PermissionsExt};

  #[test]
  fn writes_are_private_and_replace_symlinks_without_following_them() {
    let dir = std::env::temp_dir()
      .join(format!("wg-storage-{}", rand::random::<u64>()));
    fs::create_dir(&dir).unwrap();
    let outside = dir.join("outside");
    let profile = dir.join("profile.conf");
    fs::write(&outside, "keep").unwrap();
    symlink(&outside, &profile).unwrap();
    write_private(profile.to_str().unwrap(), b"secret").unwrap();
    assert_eq!(fs::read_to_string(&outside).unwrap(), "keep");
    assert_eq!(fs::read_to_string(&profile).unwrap(), "secret");
    assert_eq!(
      fs::metadata(&profile).unwrap().permissions().mode() & 0o777,
      0o600
    );
    assert!(export_private(profile.to_str().unwrap(), b"overwrite").is_err());
    assert_eq!(fs::read_to_string(&profile).unwrap(), "secret");
    fs::remove_dir_all(dir).unwrap();
  }
}
