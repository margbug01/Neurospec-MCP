use neurospec::app::{handle_cli_args, run_tauri_app};
use neurospec::utils::auto_init_logger;
use anyhow::Result;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_tauri_app();
}

fn main() -> Result<()> {
    // Initialize logging system
    if let Err(e) = auto_init_logger() {
        eprintln!("Failed to initialize logging system: {}", e);
    }

    // Handle CLI arguments
    handle_cli_args()
}


