//! Therac-25 Simulator - Main Entry Point
//!
//! Educational simulator of the Therac-25 radiation therapy machine.
//! This program demonstrates the race conditions and software failures that
//! caused patient deaths in the 1980s.
//!
//! WARNING: This simulator intentionally contains dangerous bugs for educational purposes.

#![cfg(not(target_arch = "wasm32"))]

use rstherac25::*;
use rstherac25::simulator::*;
use rstherac25::tui::TuiApp;
use rstherac25::tui_authentic::AuthenticTuiApp;
use std::sync::Arc;
use parking_lot::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Check for command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let use_authentic = args.iter().any(|arg| arg == "--authentic" || arg == "-a");

    // Print warning
    println!("\n╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║                    THERAC-25 SIMULATOR - WARNING                       ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║                                                                        ║");
    println!("║  This simulator recreates the SOFTWARE BUGS that caused REAL DEATHS   ║");
    println!("║  in the Therac-25 radiation therapy machine in the 1980s.             ║");
    println!("║                                                                        ║");
    println!("║  This is an EDUCATIONAL TOOL to demonstrate the importance of:        ║");
    println!("║    - Proper concurrent programming                                    ║");
    println!("║    - Safety-critical system design                                    ║");
    println!("║    - Race condition prevention                                        ║");
    println!("║                                                                        ║");
    println!("║  Based on Nancy Leveson's analysis: \"Medical Devices: The Therac-25\"  ║");
    println!("║                                                                        ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝\n");

    if use_authentic {
        println!("Starting Therac-25 simulator with AUTHENTIC VT100 interface...\n");
    } else {
        println!("Starting Therac-25 simulator with analytical interface...");
        println!("(Use --authentic or -a for the original VT100-style interface)\n");
    }

    // Create shared state
    let state = Arc::new(RwLock::new(TheracState::new()));
    state.write().add_log("System initialized".to_string());

    // Start concurrent tasks
    let state_clone1 = state.clone();
    let state_clone2 = state.clone();

    tokio::spawn(async move {
        treatment_monitor(state_clone1).await;
    });

    tokio::spawn(async move {
        housekeeper(state_clone2).await;
    });

    // Give tasks time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Run TUI
    if use_authentic {
        let mut app = AuthenticTuiApp::new(state.clone());
        app.run()?;
    } else {
        let mut app = TuiApp::new(state.clone());
        app.run().await?;
    }

    println!("\nTherac-25 simulator terminated.\n");

    Ok(())
}
