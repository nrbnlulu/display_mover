mod utils;

use anyhow::Ok;
use clap::{Parser, Subcommand};
use log::{Level, info};

use crate::utils::winapi::{get_monitors, get_pid_hwnd, move_window_to_monitor};

fn move_pid_windows_to_monitor(pid: isize, monitor_regex: &str) -> anyhow::Result<()> {
    let monitors = get_monitors();
    let monitor_regex =
        regex::Regex::new(monitor_regex).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
    let monitor = monitors
        .iter()
        .find(|m| monitor_regex.is_match(&m.device_name()));
    if let Some(monitor) = monitor {
        if let Some(hwnd) = get_pid_hwnd(pid)? {
            info!(
                "Moving window with PID {} to monitor: {}",
                pid,
                monitor.device_name()
            );
            move_window_to_monitor(hwnd, monitor)?;
            info!(
                "Successfully moved window with PID {} to monitor: {}",
                pid,
                monitor.device_name()
            );
        } else {
            return Err(anyhow::anyhow!("No window found for PID {}", pid));
        }
    } else {
        return Err(anyhow::anyhow!(
            "Monitor not found for regex: {}, available monitors: {:?}",
            monitor_regex,
            monitors
        ));
    }
    Ok(())
}

fn get_monitor_coords(monitor_regex: &str) -> anyhow::Result<String> {
    let monitors = get_monitors();
    let monitor_regex =
        regex::Regex::new(monitor_regex).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
    let monitor = monitors
        .iter()
        .find(|m| monitor_regex.is_match(&m.device_name()));
    if let Some(monitor) = monitor {
        let rect = monitor.virtual_work_rect();
        // return as powershell object
        let coords = format!(
            "New-Object PSObject -Property @{{DeviceName='{}'; Left={}; Top={}; Width={}; Height={}}}",
            monitor.device_name(),
            rect.left,
            rect.top,
            rect.width(),
            rect.height()
        );
        Ok(coords)
    } else {
        return Err(anyhow::anyhow!(
            "Monitor not found for regex: {}, available monitors: {:?}",
            monitor_regex,
            monitors
        ));
    }
}

#[derive(Parser)]
#[command(name = "display_mover")]
#[command(about = "Move windows to monitors by PID and monitor regex", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Move a window by PID to a monitor matching the regex
    Move {
        /// Process ID of the window to move
        #[arg(long)]
        pid: isize,
        /// Regex to match the monitor device name
        #[arg(long)]
        monitor_regex: String,
    },
    /// Get monitor coordinates
    MonitorCoords {
        /// Regex to match the monitor device name
        #[arg(long)]
        monitor_regex: String,
    },
}

fn run_cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Move { pid, monitor_regex } => {
            move_pid_windows_to_monitor(*pid, monitor_regex)?;
        },
        Commands::MonitorCoords { monitor_regex } => {
            let coords = get_monitor_coords(monitor_regex)?;
            println!("{}", coords);
        },
    }
    Ok(())
}

fn init_simpl_log() -> anyhow::Result<()> {
    simple_log::console(Level::Debug).unwrap();
    Ok(())
}

fn main() {
    init_simpl_log().unwrap();
    run_cli().unwrap();
    // let monitors = get_monitors();
    // println!("Monitors: {:?}", monitors);
    // let hwnd = get_pid_hwnd(30788).unwrap().expect("window not found for pid");
    // let first_monitor = monitors.get(1).unwrap();
    // move_window_to_monitor(hwnd, first_monitor).unwrap();
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_move_pid_windows_to_monitor() {
        // This test requires a valid PID and monitor regex to run successfully.
        // You may need to adjust the PID and regex based on your system.
        let pid = 9716; // Replace with a valid PID
        let monitor_regex = r"PHL"; // Matches 'hp' case-insensitively anywhere in the string

        let result = move_pid_windows_to_monitor(pid, monitor_regex);
        assert!(result.is_ok(), "Failed to move window: {:?}", result);
    }
}
