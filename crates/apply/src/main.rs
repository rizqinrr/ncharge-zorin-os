use clap::{Parser, Subcommand};
use ncharge_apply::apply::{execute_apply, execute_restore, EXIT_INVALID_ARGS, EXIT_OK};
use ncharge_apply::config::{read_effective_limit, read_limit_from_file, ConfigPaths};
use ncharge_apply::power::{find_supported_batteries, scan_batteries, DEFAULT_POWER_SUPPLY_ROOT};
use std::process::exit;

#[derive(Parser)]
#[command(name = "ncharge-apply")]
#[command(about = "ncharge charge limit backend helper", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply a charge limit (e.g. 60, 80, 100)
    Apply {
        /// Limit percentage between 1 and 100
        limit: u8,
        /// Temporary apply until next reboot (does not touch /etc/ncharge.conf)
        #[arg(long)]
        once: bool,
    },
    /// Restore limit from /run/ncharge.override or /etc/ncharge.conf
    Restore,
    /// Print live status (does not require root)
    Status,
}

fn main() {
    let cli = Cli::parse();
    let paths = ConfigPaths::default();

    match cli.command {
        Commands::Apply { limit, once } => {
            if !(1..=100).contains(&limit) {
                eprintln!("Error: Limit must be between 1 and 100%");
                exit(EXIT_INVALID_ARGS);
            }
            match execute_apply(limit, once, DEFAULT_POWER_SUPPLY_ROOT, &paths) {
                Ok(()) => {
                    if once {
                        println!(
                            "Charge limit temporarily applied to {}% (until restart)",
                            limit
                        );
                    } else {
                        println!("Charge limit applied and saved to {}%", limit);
                    }
                    exit(EXIT_OK);
                }
                Err(code) => {
                    eprintln!("Failed to apply charge limit (exit code {})", code);
                    exit(code);
                }
            }
        }
        Commands::Restore => match execute_restore(DEFAULT_POWER_SUPPLY_ROOT, &paths) {
            Ok(limit) => {
                println!("Restored charge limit to {}%", limit);
                exit(EXIT_OK);
            }
            Err(code) => {
                eprintln!("Failed to restore charge limit (exit code {})", code);
                exit(code);
            }
        },
        Commands::Status => {
            let bats = scan_batteries(DEFAULT_POWER_SUPPLY_ROOT);
            if bats.is_empty() {
                println!("No battery found in {}", DEFAULT_POWER_SUPPLY_ROOT);
            } else {
                for b in &bats {
                    println!("Battery: {}", b.name);
                    if let Some(cap) = b.capacity {
                        println!("  Capacity: {}%", cap);
                    }
                    if let Some(ref st) = b.status {
                        println!("  Status: {}", st);
                    }
                    match b.charge_control_end_threshold {
                        Some(t) => println!("  Hardware Threshold: {}%", t),
                        None => println!("  Hardware Threshold: Not supported"),
                    }
                }
            }

            let saved_cfg = read_limit_from_file(&paths.config_path);
            let override_cfg = read_limit_from_file(&paths.override_path);
            let (effective, is_ovr) = read_effective_limit(&paths);

            println!("\nConfiguration:");
            println!("  Saved (/etc/ncharge.conf): {:?}", saved_cfg);
            println!("  Override (/run/ncharge.override): {:?}", override_cfg);
            println!(
                "  Effective: {}%{}",
                effective,
                if is_ovr { " (temporary override)" } else { "" }
            );

            let supported = find_supported_batteries(DEFAULT_POWER_SUPPLY_ROOT);
            if supported.is_empty() {
                exit(ncharge_apply::apply::EXIT_SYSFS_UNAVAILABLE);
            }
            exit(EXIT_OK);
        }
    }
}
