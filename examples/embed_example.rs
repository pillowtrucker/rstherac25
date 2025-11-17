//! Example of embedding rstherac25 in another application
//!
//! This example demonstrates how to use rstherac25 as a library,
//! creating state, manipulating it, and spawning background tasks.
//!
//! Run with: cargo run --example embed_example --features standalone

use rstherac25::*;

#[cfg(feature = "standalone")]
#[tokio::main]
async fn main() {
    println!("=== rstherac25 Embedding Example ===\n");

    // Step 1: Create state
    println!("1. Creating Therac-25 state...");
    let state = create_therac_state();
    println!("   Initial phase: {}", get_phase(&state));
    println!();

    // Step 2: Spawn background tasks
    println!("2. Spawning background tasks (treatment monitor & housekeeper)...");
    let tasks = spawn_treatment_tasks(state.clone());
    println!("   Tasks spawned successfully");
    println!();

    // Give tasks time to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Step 3: Simulate operator actions
    println!("3. Simulating operator data entry...");

    // Set mode to X-ray
    handle_mode_input(state.clone(), BeamType::XRay);
    println!("   - Set mode to X-Ray");

    // Set gantry angle
    handle_gantry_input(state.clone(), 180);
    println!("   - Set gantry to 180 degrees");

    // Set field size
    handle_field_size_input(state.clone(), 15.0, 15.0);
    println!("   - Set field size to 15x15 cm");

    // Set dose
    handle_dose_input(state.clone(), 200.0);
    println!("   - Set dose to 200 cGy");
    println!();

    // Step 4: Complete data entry
    println!("4. Completing data entry...");
    complete_data_entry(state.clone());

    // Wait for setup to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    println!("   Current phase: {}", get_phase(&state));
    println!();

    // Step 5: Show state
    println!("5. Current state:");
    {
        let s = state.read();
        println!("   Console MEOS: {} @ {}", s.console_meos.beam_type, s.console_meos.beam_energy);
        println!("   Hardware MEOS: {} @ {}", s.hardware_meos.beam_type, s.hardware_meos.beam_energy);
        println!("   Collimator: {}", s.hardware_meos.collimator);
        println!("   Hardware is safe: {}", s.hardware_meos.is_safe());
        println!("   Dose delivered: {:.1}/{:.1} cGy", s.dose_delivered, s.dose_target);

        if !s.log.is_empty() {
            println!("\n   Recent log entries:");
            for entry in s.log.iter().rev().take(5).rev() {
                println!("     {}", entry);
            }
        }
    }
    println!();

    // Step 6: Check if ready for treatment
    println!("6. Treatment readiness:");
    if can_treat(&state) {
        println!("   ✓ System is ready for treatment");

        // Optionally start treatment
        if handle_treat_command(state.clone()) {
            println!("   - Treatment started!");

            // Wait a bit for treatment to progress
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let s = state.read();
            println!("   - Dose delivered: {:.1} cGy", s.dose_delivered);
        }
    } else {
        println!("   ✗ System is not ready for treatment");
        println!("   Current phase: {}", get_phase(&state));
    }
    println!();

    // Step 7: Cleanup
    println!("7. Cleaning up tasks...");
    cleanup_tasks(tasks);
    println!("   Tasks terminated");
    println!();

    println!("=== Example Complete ===");
}

#[cfg(not(feature = "standalone"))]
fn main() {
    eprintln!("This example requires the 'standalone' feature.");
    eprintln!("Run: cargo run --example embed_example --features standalone");
    std::process::exit(1);
}
