//! # rstherac25 - Therac-25 Radiation Therapy Simulator
//!
//! This is an educational simulator that recreates the control software and race conditions
//! of the infamous Therac-25 radiation therapy machine. The Therac-25 caused several patient
//! deaths in the 1980s due to software failures, particularly race conditions in concurrent
//! state management.
//!
//! ## Safety Warning
//! This simulator intentionally contains the race conditions and bugs that caused real-world
//! harm. It is for educational purposes only to demonstrate the importance of proper concurrent
//! programming and safety-critical system design.
//!
//! ## Architecture
//! The simulator models three concurrent tasks:
//! 1. Treatment Monitor - manages the treatment state machine
//! 2. Housekeeper - synchronizes hardware collimator positions
//! 3. External Interface - handles operator input
//!
//! The critical race condition occurs when the treatment monitor checks if console parameters
//! match hardware parameters, but doesn't lock state during the check and subsequent beam
//! delivery, allowing the housekeeper to modify hardware state in between.
//!
//! ## Usage
//!
//! ### As a library (embeddable):
//! ```ignore
//! use rstherac25::*;
//!
//! // Create state
//! let state = create_therac_state();
//!
//! // For standalone async applications:
//! #[cfg(feature = "standalone")]
//! {
//!     let tasks = spawn_treatment_tasks(state.clone());
//!     // ... later
//!     cleanup_tasks(tasks);
//! }
//! ```
//!
//! ### As a standalone application:
//! ```bash
//! cargo run --features standalone
//! ```

use std::sync::Arc;
use parking_lot::RwLock;

// Re-export core modules
pub mod state;
pub mod simulator;
pub mod input;

// Optional rendering module (only with "tui-render" feature)
#[cfg(feature = "tui-render")]
pub mod render;

// TUI modules for standalone mode
#[cfg(feature = "standalone")]
pub mod tui;

#[cfg(feature = "standalone")]
pub mod tui_authentic;

// WASM module
#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export commonly used types from state module
pub use state::{
    TheracState, SharedTheracState, TPhase, BeamType, BeamEnergy,
    CollimatorPosition, Meos, TreatmentParams,
};

// Re-export simulator functions
#[cfg(feature = "standalone")]
pub use simulator::{
    spawn_treatment_tasks, cleanup_tasks, TheracTaskHandles,
    treatment_monitor, housekeeper,
};

// Re-export input helpers
pub use input::{
    InputField, handle_mode_input, handle_energy_input, handle_gantry_input,
    handle_field_size_input, handle_dose_input, handle_treat_command,
    handle_reset_command, handle_setup_test_command, is_data_entry_complete,
    can_treat, get_phase, complete_data_entry,
};

// Re-export render function when available
#[cfg(feature = "tui-render")]
pub use render::render_therac25;

/// Initialize a new Therac-25 instance
pub fn create_therac_state() -> SharedTheracState {
    Arc::new(RwLock::new(TheracState::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_state() {
        let state = create_therac_state();
        let s = state.read();
        assert_eq!(s.phase, TPhase::Reset);
    }

    #[test]
    fn test_can_treat() {
        let state = create_therac_state();
        assert!(!can_treat(&state));

        state.write().phase = TPhase::SetupDone;
        assert!(can_treat(&state));
    }
}
