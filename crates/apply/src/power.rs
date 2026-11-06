use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_POWER_SUPPLY_ROOT: &str = "/sys/class/power_supply";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryInfo {
    pub name: String,
    pub path: PathBuf,
    pub capacity: Option<u8>,
    pub status: Option<String>,
    pub charge_control_end_threshold: Option<u8>,
}

pub fn scan_batteries<P: AsRef<Path>>(root: P) -> Vec<BatteryInfo> {
    let root = root.as_ref();
    let mut batteries = Vec::new();

    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return batteries,
    };

    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|res| res.ok().map(|e| e.path()))
        .filter(|p| {
            if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                file_name.starts_with("BAT")
            } else {
                false
            }
        })
        .collect();

    dirs.sort();

    for dir in dirs {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("BAT")
            .to_string();

        let capacity = fs::read_to_string(dir.join("capacity"))
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok());

        let status = fs::read_to_string(dir.join("status"))
            .ok()
            .map(|s| s.trim().to_string());

        let charge_control_end_threshold =
            fs::read_to_string(dir.join("charge_control_end_threshold"))
                .ok()
                .and_then(|s| s.trim().parse::<u8>().ok());

        batteries.push(BatteryInfo {
            name,
            path: dir,
            capacity,
            status,
            charge_control_end_threshold,
        });
    }

    batteries
}

pub fn find_supported_batteries<P: AsRef<Path>>(root: P) -> Vec<BatteryInfo> {
    scan_batteries(root)
        .into_iter()
        .filter(|b| b.charge_control_end_threshold.is_some())
        .collect()
}

pub fn write_battery_threshold(battery_path: &Path, limit: u8) -> std::io::Result<()> {
    let file = battery_path.join("charge_control_end_threshold");
    fs::write(file, format!("{}\n", limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scan_and_probe() {
        let dir = tempdir().unwrap();
        let bat0 = dir.path().join("BAT0");
        let ac0 = dir.path().join("AC0");
        fs::create_dir(&bat0).unwrap();
        fs::create_dir(&ac0).unwrap();

        fs::write(bat0.join("capacity"), "74\n").unwrap();
        fs::write(bat0.join("status"), "Charging\n").unwrap();
        fs::write(bat0.join("charge_control_end_threshold"), "80\n").unwrap();

        let all = scan_batteries(dir.path());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "BAT0");
        assert_eq!(all[0].capacity, Some(74));
        assert_eq!(all[0].status.as_deref(), Some("Charging"));
        assert_eq!(all[0].charge_control_end_threshold, Some(80));

        let supported = find_supported_batteries(dir.path());
        assert_eq!(supported.len(), 1);

        write_battery_threshold(&bat0, 60).unwrap();
        let val = fs::read_to_string(bat0.join("charge_control_end_threshold")).unwrap();
        assert_eq!(val.trim(), "60");
    }
}
