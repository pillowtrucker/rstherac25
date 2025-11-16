//! Therac-25 simulator core logic
//!
//! This module implements the concurrent tasks that manage the Therac-25 treatment process:
//! - Treatment monitor: manages the state machine through treatment phases
//! - Housekeeper: synchronizes hardware collimator position with console settings
//! - The critical race condition in zap_the_specimen()

use crate::*;
use std::time::Duration;
use tokio::time::sleep;
use rand::Rng;

/// Treatment monitor task
/// Manages the treatment state machine, cycling through phases
pub async fn treatment_monitor(state: SharedTheracState) {
    loop {
        sleep(Duration::from_micros(1666)).await; // ~60Hz polling

        let current_phase = {
            let s = state.read();
            s.phase
        };

        match current_phase {
            TPhase::Reset => handle_reset(state.clone()).await,
            TPhase::DataEntry => handle_data_entry(state.clone()).await,
            TPhase::SetupTest => handle_setup_test(state.clone()).await,
            TPhase::SetupDone => handle_setup_done(state.clone()).await,
            TPhase::PatientTreatment => handle_patient_treatment(state.clone()).await,
            TPhase::PauseTreatment => handle_pause_treatment(state.clone()).await,
            TPhase::TerminateTreatment => handle_terminate_treatment(state.clone()).await,
            TPhase::DateTimeIdChanges => handle_datetime_changes(state.clone()).await,
        }
    }
}

/// Housekeeper task
/// Continuously synchronizes collimator position between console and hardware settings
/// This task runs concurrently and can modify hardware_meos, creating the race condition
pub async fn housekeeper(state: SharedTheracState) {
    loop {
        sleep(Duration::from_micros(1666)).await; // ~60Hz polling

        sync_collimator(state.clone()).await;
    }
}

/// Synchronize collimator position and other hardware parameters
/// This is the concurrent task that creates the race condition with zap_the_specimen
async fn sync_collimator(state: SharedTheracState) {
    let needs_sync = {
        let s = state.read();
        // Only sync if not in critical treatment phase
        s.phase != TPhase::PatientTreatment && s.console_meos.needs_collimator_sync()
    };

    if needs_sync {
        let (console_beam_type, current_collimator) = {
            let s = state.read();
            (s.console_meos.beam_type, s.hardware_meos.collimator)
        };

        // Simulate collimator movement delay
        if current_collimator != CollimatorPosition::Transitioning {
            {
                let mut s = state.write();
                s.hardware_meos.collimator = CollimatorPosition::Transitioning;
                s.add_log("Collimator moving...".to_string());
            }
        }

        // Simulate physical movement time (magnet hysteresis + mechanical delays)
        // Real Therac-25 had ~100ms, but we use 800ms to make the race window
        // more educational while still being realistic
        sleep(Duration::from_millis(800)).await;

        // Move to target position
        let target_position = match console_beam_type {
            BeamType::XRay => CollimatorPosition::InPosition,
            BeamType::Electron => CollimatorPosition::OutOfPosition,
            BeamType::Undefined => CollimatorPosition::OutOfPosition,
        };

        {
            let mut s = state.write();
            s.hardware_meos.collimator = target_position;
            // Also sync beam type and energy during collimator movement
            s.hardware_meos.beam_type = s.console_meos.beam_type;
            s.hardware_meos.beam_energy = s.console_meos.beam_energy;
            let beam_type = s.hardware_meos.beam_type;
            let beam_energy = s.hardware_meos.beam_energy;
            s.add_log(format!("Hardware synced: {} @ {} with collimator {}",
                beam_type,
                beam_energy,
                target_position));
        }
    }

    // Also sync other hardware parameters (gantry, field size, etc.)
    // This happens continuously and more slowly
    let params_need_sync = {
        let s = state.read();
        s.phase != TPhase::PatientTreatment && s.console_params != s.hardware_params
    };

    if params_need_sync {
        // Simulate mechanical movement delays for gantry, collimator rotation, etc.
        sleep(Duration::from_millis(200)).await;

        let mut s = state.write();
        s.hardware_params = s.console_params;
    }
}

/// Handle reset phase
async fn handle_reset(state: SharedTheracState) {
    sleep(Duration::from_millis(100)).await;

    let mut s = state.write();
    s.phase = TPhase::DataEntry;
    s.add_log("Entering data entry mode".to_string());
}

/// Handle data entry phase
async fn handle_data_entry(state: SharedTheracState) {
    let data_complete = {
        let s = state.read();
        s.data_entry_complete
    };

    if data_complete {
        let mut s = state.write();
        s.phase = TPhase::SetupTest;
        s.class3 = 0;
        s.add_log("Data entry complete, starting setup test".to_string());
    }
}

/// Handle setup test phase
async fn handle_setup_test(state: SharedTheracState) {
    sleep(Duration::from_millis(50)).await;

    let mut s = state.write();
    s.class3 = s.class3.wrapping_add(1);

    // After several iterations, move to setup done
    if s.class3 > 10 {
        s.phase = TPhase::SetupDone;
        s.add_log("Setup test complete".to_string());
    }
}

/// Handle setup done phase
async fn handle_setup_done(_state: SharedTheracState) {
    // Wait for external trigger to start treatment
    // In the TUI, this will be triggered by operator action
}

/// Handle patient treatment phase
/// This is where the critical beam delivery happens
async fn handle_patient_treatment(state: SharedTheracState) {
    zap_the_specimen(state.clone()).await;
}

/// Handle pause treatment phase
async fn handle_pause_treatment(_state: SharedTheracState) {
    // Treatment is paused, waiting for operator action
}

/// Handle terminate treatment phase
async fn handle_terminate_treatment(state: SharedTheracState) {
    sleep(Duration::from_millis(100)).await;

    let mut s = state.write();
    let dose_delivered = s.dose_delivered;
    let dose_target = s.dose_target;
    s.add_log(format!(
        "Treatment terminated. Dose delivered: {:.1}/{:.1} cGy",
        dose_delivered, dose_target
    ));
    s.phase = TPhase::Reset;
}

/// Handle date/time/ID changes phase
async fn handle_datetime_changes(state: SharedTheracState) {
    sleep(Duration::from_millis(100)).await;

    let mut s = state.write();
    s.phase = TPhase::DataEntry;
}

/// ZAP THE SPECIMEN
/// This function contains the CRITICAL RACE CONDITION that caused real-world incidents
///
/// The bug: State is read outside the critical section, then checked inside.
/// Between the read and the check, the housekeeper task can modify hardware_meos,
/// causing a mismatch that results in either:
/// 1. MALFUNCTION 54 if detected
/// 2. Massive overdose if the beam fires with wrong collimator position
///
/// In the original Therac-25, this happened when operators:
/// 1. Entered X-ray mode
/// 2. Quickly corrected to Electron mode before pressing the beam trigger
/// 3. The software accepted Electron parameters (no flatness filter needed)
/// 4. But the hardware was still in X-ray mode with high-energy beam
/// 5. Without the flatness filter to spread the beam, patients received 100x the intended dose
pub async fn zap_the_specimen(state: SharedTheracState) {
    // Simulate random hardware reliability issues
    let really_good_number: u32 = rand::thread_rng().gen_range(12..=53);

    // CRITICAL BUG: Read state outside the atomic operation
    // This creates a check-then-act race condition
    let (console_meos, hardware_meos) = {
        let s = state.read();
        (s.console_meos, s.hardware_meos)
    };

    // Small delay to increase chance of race condition manifesting
    sleep(Duration::from_micros(100)).await;

    // CRITICAL SECTION: Check if parameters match
    // But hardware_meos might have changed since we read it above!
    let mut s = state.write();

    // Check for parameter mismatch
    if console_meos != hardware_meos {
        // MALFUNCTION 54: Parameter mismatch detected
        s.malfunction_count += 1;
        s.phase = TPhase::PauseTreatment;
        let malfunction_msg = format!("MALFUNCTION 54 - Parameter mismatch (occurrence #{}) - Console: {:?}/{}, Hardware: {:?}/{}",
            s.malfunction_count,
            console_meos.beam_type,
            console_meos.collimator,
            hardware_meos.beam_type,
            hardware_meos.collimator);
        s.last_malfunction = Some(malfunction_msg.clone());
        s.add_log(malfunction_msg);
        return;
    }

    // Check if hardware configuration is unsafe
    if !s.hardware_meos.is_safe() {
        // CRITICAL SAFETY VIOLATION
        // Delivering beam with wrong collimator position!
        s.malfunction_count += 1;

        let dose_multiplier = match s.hardware_meos.beam_type {
            BeamType::XRay if s.hardware_meos.collimator == CollimatorPosition::OutOfPosition => {
                // X-ray mode without flatness filter = MASSIVE overdose
                // The flatness filter normally spreads the beam over a large area
                // Without it, all energy is concentrated in a small spot
                100.0
            },
            BeamType::Electron if s.hardware_meos.collimator == CollimatorPosition::InPosition => {
                // Electron mode with filter = underdose (filter blocks electrons)
                0.1
            },
            _ => 1.0,
        };

        let dose_this_pulse = calculate_dose(&s.hardware_meos) * dose_multiplier;
        s.dose_delivered += dose_this_pulse;

        s.phase = TPhase::PauseTreatment;
        let dose_delivered = s.dose_delivered;
        let dose_target = s.dose_target;
        let malfunction_msg = format!(
            "CRITICAL SAFETY VIOLATION! Beam fired with unsafe configuration! Dose multiplier: {:.1}x - Delivered {:.1} cGy this pulse (total: {:.1}/{:.1} cGy)",
            dose_multiplier, dose_this_pulse, dose_delivered, dose_target
        );
        s.last_malfunction = Some(malfunction_msg.clone());
        s.add_log(malfunction_msg);
        return;
    }

    // Simulate random hardware malfunctions
    if really_good_number > 22 {
        s.malfunction_count += 1;
        s.phase = TPhase::PauseTreatment;
        let malfunction_msg = format!("MALFUNCTION {} - Random hardware fault", really_good_number);
        s.last_malfunction = Some(malfunction_msg.clone());
        s.add_log(malfunction_msg);
        return;
    }

    // Normal beam delivery
    let dose_this_pulse = calculate_dose(&s.hardware_meos);
    s.dose_delivered += dose_this_pulse;

    let dose_delivered = s.dose_delivered;
    let dose_target = s.dose_target;
    s.add_log(format!(
        "Beam delivered: {:.2} cGy (total: {:.1}/{:.1} cGy)",
        dose_this_pulse, dose_delivered, dose_target
    ));

    // Check if target dose reached
    if s.dose_delivered >= s.dose_target {
        s.phase = TPhase::TerminateTreatment;
        s.add_log("Target dose reached".to_string());
    }
}

/// Calculate dose for a single beam pulse
/// Dose depends on beam type and energy level
fn calculate_dose(meos: &Meos) -> f64 {
    let base_dose = match meos.beam_energy {
        BeamEnergy::E5 => 2.0,
        BeamEnergy::E10 => 4.0,
        BeamEnergy::E15 => 6.0,
        BeamEnergy::E20 => 8.0,
        BeamEnergy::E25 => 10.0,
    };

    // X-ray mode delivers dose over larger area (with flatness filter)
    match meos.beam_type {
        BeamType::XRay => base_dose * 0.8,
        BeamType::Electron => base_dose,
        BeamType::Undefined => 0.0,
    }
}

/// Start treatment
pub fn start_treatment(state: SharedTheracState) {
    let mut s = state.write();
    if s.phase == TPhase::SetupDone {
        s.phase = TPhase::PatientTreatment;
        s.add_log("Starting patient treatment".to_string());
    }
}

/// Stop treatment
pub fn stop_treatment(state: SharedTheracState) {
    let mut s = state.write();
    if s.phase == TPhase::PatientTreatment {
        s.phase = TPhase::PauseTreatment;
        s.add_log("Treatment paused by operator".to_string());
    }
}

/// Resume treatment
pub fn resume_treatment(state: SharedTheracState) {
    let mut s = state.write();
    if s.phase == TPhase::PauseTreatment {
        s.phase = TPhase::PatientTreatment;
        s.last_malfunction = None;
        s.add_log("Treatment resumed".to_string());
    }
}

/// Complete data entry
/// Note: This does NOT immediately sync hardware - that happens asynchronously via the housekeeper
/// This is intentional and creates the race condition!
pub fn complete_data_entry(state: SharedTheracState) {
    let mut s = state.write();
    if s.phase == TPhase::DataEntry {
        s.data_entry_complete = true;
        s.editing_taking_place = false;
        // DO NOT copy console settings to hardware here - let the housekeeper do it
        // This creates the race condition window
        s.add_log("Data entry complete - hardware sync pending".to_string());
    }
}

/// Update console MEOS (operator input)
pub fn update_console_meos(state: SharedTheracState, meos: Meos) {
    let mut s = state.write();
    if s.phase == TPhase::DataEntry || s.phase == TPhase::SetupTest {
        s.console_meos = meos;
        s.add_log(format!(
            "Console updated: {} @ {} with collimator {}",
            meos.beam_type, meos.beam_energy, meos.collimator
        ));
    }
}

/// Generate random treatment parameters
pub fn generate_random_parameters() -> Meos {
    let mut rng = rand::thread_rng();

    let beam_type = if rng.gen_bool(0.5) {
        BeamType::XRay
    } else {
        BeamType::Electron
    };

    let beam_energy = match rng.gen_range(0..5) {
        0 => BeamEnergy::E5,
        1 => BeamEnergy::E10,
        2 => BeamEnergy::E15,
        3 => BeamEnergy::E20,
        _ => BeamEnergy::E25,
    };

    // Collimator should match beam type for safe operation
    let collimator = match beam_type {
        BeamType::XRay => CollimatorPosition::InPosition,
        BeamType::Electron => CollimatorPosition::OutOfPosition,
        BeamType::Undefined => CollimatorPosition::OutOfPosition,
    };

    Meos {
        beam_type,
        beam_energy,
        collimator,
    }
}

/// Generate random parameters that might trigger the race condition
/// This simulates an operator quickly changing parameters
pub fn generate_race_condition_parameters(current: Meos) -> Meos {
    // Flip the beam type but keep the collimator position wrong
    let new_beam_type = match current.beam_type {
        BeamType::XRay => BeamType::Electron,
        BeamType::Electron => BeamType::XRay,
        BeamType::Undefined => BeamType::Electron,
    };

    Meos {
        beam_type: new_beam_type,
        beam_energy: current.beam_energy,
        // Keep old collimator position - this is what triggers the bug!
        collimator: current.collimator,
    }
}
