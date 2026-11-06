use crate::config::{
    read_effective_limit, remove_override_file, write_config_atomic, ConfigPaths, DEFAULT_LIMIT,
};
use crate::power::{find_supported_batteries, write_battery_threshold};
use std::path::Path;

pub const EXIT_OK: i32 = 0;
pub const EXIT_INVALID_ARGS: i32 = 1;
pub const EXIT_SYSFS_UNAVAILABLE: i32 = 2;
pub const EXIT_WRITE_FAILED: i32 = 3;

pub fn execute_apply<P: AsRef<Path>>(
    limit: u8,
    once: bool,
    power_root: P,
    paths: &ConfigPaths,
) -> Result<(), i32> {
    if !(1..=100).contains(&limit) {
        return Err(EXIT_INVALID_ARGS);
    }

    let supported = find_supported_batteries(power_root);
    if supported.is_empty() {
        return Err(EXIT_SYSFS_UNAVAILABLE);
    }

    for bat in &supported {
        if write_battery_threshold(&bat.path, limit).is_err() {
            return Err(EXIT_WRITE_FAILED);
        }
    }

    if once {
        if write_config_atomic(&paths.override_path, limit).is_err() {
            return Err(EXIT_WRITE_FAILED);
        }
    } else {
        remove_override_file(&paths.override_path);
        if write_config_atomic(&paths.config_path, limit).is_err() {
            return Err(EXIT_WRITE_FAILED);
        }
    }

    Ok(())
}

pub fn execute_restore<P: AsRef<Path>>(power_root: P, paths: &ConfigPaths) -> Result<u8, i32> {
    let supported = find_supported_batteries(power_root);
    if supported.is_empty() {
        return Err(EXIT_SYSFS_UNAVAILABLE);
    }

    let (limit, is_override) = read_effective_limit(paths);

    // Pastikan config ada bila restore berjalan pertama kali
    if !paths.config_path.exists() && !is_override {
        let _ = write_config_atomic(&paths.config_path, DEFAULT_LIMIT);
    }

    for bat in &supported {
        if write_battery_threshold(&bat.path, limit).is_err() {
            return Err(EXIT_WRITE_FAILED);
        }
    }

    Ok(limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::read_limit_from_file;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_apply_and_restore() {
        let dir = tempdir().unwrap();
        let sys_root = dir.path().join("power_supply");
        let bat0 = sys_root.join("BAT0");
        fs::create_dir_all(&bat0).unwrap();
        fs::write(bat0.join("charge_control_end_threshold"), "100\n").unwrap();

        let cfg_dir = dir.path().join("etc");
        let run_dir = dir.path().join("run");
        fs::create_dir_all(&cfg_dir).unwrap();
        fs::create_dir_all(&run_dir).unwrap();

        let paths = ConfigPaths {
            config_path: cfg_dir.join("ncharge.conf"),
            override_path: run_dir.join("ncharge.override"),
        };

        // 1. Apply 80 (permanen)
        let res = execute_apply(80, false, &sys_root, &paths);
        assert_eq!(res, Ok(()));
        assert_eq!(
            fs::read_to_string(bat0.join("charge_control_end_threshold"))
                .unwrap()
                .trim(),
            "80"
        );
        assert_eq!(read_limit_from_file(&paths.config_path), Some(80));
        assert!(!paths.override_path.exists());

        // 2. Apply 100 --once (sementara)
        let res2 = execute_apply(100, true, &sys_root, &paths);
        assert_eq!(res2, Ok(()));
        assert_eq!(
            fs::read_to_string(bat0.join("charge_control_end_threshold"))
                .unwrap()
                .trim(),
            "100"
        );
        // config tetap 80
        assert_eq!(read_limit_from_file(&paths.config_path), Some(80));
        assert_eq!(read_limit_from_file(&paths.override_path), Some(100));

        // 3. Restore saat override ada
        fs::write(bat0.join("charge_control_end_threshold"), "100\n").unwrap();
        let rest1 = execute_restore(&sys_root, &paths);
        assert_eq!(rest1, Ok(100));

        // 4. Simulasi reboot: /run hilang
        fs::remove_file(&paths.override_path).unwrap();
        let rest2 = execute_restore(&sys_root, &paths);
        assert_eq!(rest2, Ok(80));
        assert_eq!(
            fs::read_to_string(bat0.join("charge_control_end_threshold"))
                .unwrap()
                .trim(),
            "80"
        );
    }
}
