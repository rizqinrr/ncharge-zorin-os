use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_PATH: &str = "/etc/ncharge.conf";
pub const DEFAULT_OVERRIDE_PATH: &str = "/run/ncharge.override";
pub const DEFAULT_LIMIT: u8 = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargeConfig {
    pub limit: u8,
}

pub struct ConfigPaths {
    pub config_path: PathBuf,
    pub override_path: PathBuf,
}

impl Default for ConfigPaths {
    fn default() -> Self {
        Self {
            config_path: PathBuf::from(DEFAULT_CONFIG_PATH),
            override_path: PathBuf::from(DEFAULT_OVERRIDE_PATH),
        }
    }
}

pub fn parse_config_str(content: &str) -> Option<u8> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = trimmed.split_once('=') {
            if key.trim() == "limit" {
                if let Ok(limit) = val.trim().parse::<u8>() {
                    if (1..=100).contains(&limit) {
                        return Some(limit);
                    }
                }
            }
        }
    }
    None
}

pub fn read_limit_from_file<P: AsRef<Path>>(path: P) -> Option<u8> {
    match fs::read_to_string(path) {
        Ok(content) => parse_config_str(&content),
        Err(_) => None,
    }
}

pub fn read_effective_limit(paths: &ConfigPaths) -> (u8, bool) {
    if let Some(limit) = read_limit_from_file(&paths.override_path) {
        return (limit, true);
    }
    if let Some(limit) = read_limit_from_file(&paths.config_path) {
        return (limit, false);
    }
    (DEFAULT_LIMIT, false)
}

pub fn write_config_atomic<P: AsRef<Path>>(path: P, limit: u8) -> io::Result<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let tmp_path = parent.join(format!(".ncharge.tmp.{}", std::process::id()));
    let content = format!("# ncharge configuration\nlimit = {}\n", limit);

    fs::write(&tmp_path, content)?;

    let perms = fs::Permissions::from_mode(0o644);
    if let Err(e) = fs::set_permissions(&tmp_path, perms) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    Ok(())
}

pub fn remove_override_file<P: AsRef<Path>>(override_path: P) {
    let p = override_path.as_ref();
    if p.exists() {
        let _ = fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_valid() {
        let text = "# comment\n\nlimit = 80\n";
        assert_eq!(parse_config_str(text), Some(80));

        let text2 = "limit=60";
        assert_eq!(parse_config_str(text2), Some(60));
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(parse_config_str("limit = 0"), None);
        assert_eq!(parse_config_str("limit = 101"), None);
        assert_eq!(parse_config_str("limit = abc"), None);
        assert_eq!(parse_config_str("unknown = 80"), None);
        assert_eq!(parse_config_str(""), None);
    }

    #[test]
    fn test_atomic_write_and_override() {
        let dir = tempdir().unwrap();
        let cfg_path = dir.path().join("ncharge.conf");
        let ovr_path = dir.path().join("ncharge.override");

        let paths = ConfigPaths {
            config_path: cfg_path.clone(),
            override_path: ovr_path.clone(),
        };

        // Belum ada file, fallback ke default 80
        assert_eq!(read_effective_limit(&paths), (80, false));

        // Tulis config 60
        write_config_atomic(&cfg_path, 60).unwrap();
        assert_eq!(read_effective_limit(&paths), (60, false));

        // File permissions 0644
        let metadata = fs::metadata(&cfg_path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o644);

        // Override aktif 100
        write_config_atomic(&ovr_path, 100).unwrap();
        assert_eq!(read_effective_limit(&paths), (100, true));

        // Hapus override -> kembali ke 60
        remove_override_file(&ovr_path);
        assert_eq!(read_effective_limit(&paths), (60, false));
    }
}
