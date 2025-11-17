//! Input handling helpers for the Therac-25 simulator
//!
//! This module provides utility functions for handling user input and
//! updating the simulator state.

use crate::state::{SharedTheracState, BeamType, BeamEnergy, TPhase};

/// Input field identifier for data entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    Mode,
    Energy,
    Gantry,
    FieldSize,
    Dose,
    Command,
}

/// Handle mode selection (X-ray or Electron)
pub fn handle_mode_input(state: SharedTheracState, mode: BeamType) {
    let mut s = state.write();
    s.console_meos.beam_type = mode;

    // Auto-set energy for X-ray mode (as per real Therac-25)
    if mode == BeamType::XRay {
        s.console_meos.beam_energy = BeamEnergy::E25;
    }

    s.add_log(format!("[CONSOLE] Mode set to {:?}", mode));
}

/// Handle energy selection
pub fn handle_energy_input(state: SharedTheracState, energy: BeamEnergy) {
    let mut s = state.write();
    s.console_meos.beam_energy = energy;
    s.add_log(format!("[CONSOLE] Energy set to {}", energy));
}

/// Handle gantry angle input
pub fn handle_gantry_input(state: SharedTheracState, angle: u16) {
    let mut s = state.write();
    s.console_params.gantry_angle = angle;
    s.add_log(format!("[CONSOLE] Gantry angle set to {} deg", angle));
}

/// Handle field size input
pub fn handle_field_size_input(state: SharedTheracState, x: f32, y: f32) {
    let mut s = state.write();
    s.console_params.field_size_x = x;
    s.console_params.field_size_y = y;
    s.add_log(format!("[CONSOLE] Field size set to {}x{} cm", x, y));
}

/// Handle dose target input
pub fn handle_dose_input(state: SharedTheracState, dose: f64) {
    let mut s = state.write();
    s.dose_target = dose;
    s.add_log(format!("[CONSOLE] Dose target set to {} cGy", dose));
}

/// Handle treat command - start treatment
pub fn handle_treat_command(state: SharedTheracState) -> bool {
    let mut s = state.write();
    if s.phase == TPhase::SetupDone {
        s.phase = TPhase::PatientTreatment;
        s.add_log("[OPERATOR] Treatment started".to_string());
        true
    } else {
        s.add_log("[OPERATOR] Cannot start treatment - setup not complete".to_string());
        false
    }
}

/// Handle reset command
pub fn handle_reset_command(state: SharedTheracState) {
    let mut s = state.write();
    s.reset();
}

/// Handle setup test command
pub fn handle_setup_test_command(state: SharedTheracState) {
    let mut s = state.write();
    if s.phase == TPhase::DataEntry && s.data_entry_complete {
        s.phase = TPhase::SetupTest;
        s.add_log("[OPERATOR] Setup test initiated".to_string());
    }
}

/// Check if data entry is complete
pub fn is_data_entry_complete(state: SharedTheracState) -> bool {
    let s = state.read();
    s.data_entry_complete
}

/// Check if currently in a state that can start treatment
pub fn can_treat(state: &SharedTheracState) -> bool {
    let s = state.read();
    s.phase == TPhase::SetupDone
}

/// Get current phase
pub fn get_phase(state: &SharedTheracState) -> TPhase {
    state.read().phase
}

/// Mark data entry as complete
pub fn complete_data_entry(state: SharedTheracState) {
    let mut s = state.write();
    if s.phase == TPhase::DataEntry {
        s.data_entry_complete = true;
        s.editing_taking_place = false;
        // DO NOT copy console settings to hardware here - let the housekeeper do it
        // This is part of the race condition design
        s.add_log("[CONSOLE] Data entry complete".to_string());
    }
}
