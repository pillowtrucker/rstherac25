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

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub mod simulator;

#[cfg(not(target_arch = "wasm32"))]
pub mod tui;

#[cfg(not(target_arch = "wasm32"))]
pub mod tui_authentic;

#[cfg(feature = "wasm")]
pub mod wasm;

/// Beam type for radiation therapy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeamType {
    /// X-ray mode - requires flatness filter (turntable in position)
    XRay,
    /// Electron beam mode - direct beam (turntable out of position)
    Electron,
    /// Undefined/transitioning state
    Undefined,
}

impl Default for BeamType {
    fn default() -> Self {
        Self::Undefined
    }
}

impl std::fmt::Display for BeamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeamType::XRay => write!(f, "X-Ray"),
            BeamType::Electron => write!(f, "Electron"),
            BeamType::Undefined => write!(f, "Undefined"),
        }
    }
}

/// Beam energy level in MeV (Mega-electron volts)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeamEnergy {
    /// 5 MeV
    E5,
    /// 10 MeV
    E10,
    /// 15 MeV
    E15,
    /// 20 MeV
    E20,
    /// 25 MeV
    E25,
}

impl Default for BeamEnergy {
    fn default() -> Self {
        Self::E10
    }
}

impl std::fmt::Display for BeamEnergy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeamEnergy::E5 => write!(f, "5 MeV"),
            BeamEnergy::E10 => write!(f, "10 MeV"),
            BeamEnergy::E15 => write!(f, "15 MeV"),
            BeamEnergy::E20 => write!(f, "20 MeV"),
            BeamEnergy::E25 => write!(f, "25 MeV"),
        }
    }
}

/// Collimator position (turntable position)
/// In the real Therac-25, the turntable rotated to position different beam modification
/// devices (flatness filter for X-ray mode, or out of the way for electron mode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollimatorPosition {
    /// Flatness filter in beam path (required for X-ray mode)
    InPosition,
    /// No filter in beam path (electron mode)
    OutOfPosition,
    /// Moving between positions
    Transitioning,
}

impl Default for CollimatorPosition {
    fn default() -> Self {
        Self::OutOfPosition
    }
}

impl std::fmt::Display for CollimatorPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollimatorPosition::InPosition => write!(f, "In Position"),
            CollimatorPosition::OutOfPosition => write!(f, "Out"),
            CollimatorPosition::Transitioning => write!(f, "Moving..."),
        }
    }
}

/// MEOS - Mode/Energy/Offset Structure
/// Represents the complete configuration for a treatment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meos {
    /// Beam type and energy level
    pub beam_type: BeamType,
    pub beam_energy: BeamEnergy,
    /// Collimator/turntable position
    pub collimator: CollimatorPosition,
}

/// Treatment parameters - additional settings beyond MEOS
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TreatmentParams {
    /// Gantry angle in degrees (0-360)
    pub gantry_angle: u16,
    /// Collimator rotation angle in degrees (0-360)
    pub collimator_angle: u16,
    /// Field size X in cm (0-40)
    pub field_size_x: f32,
    /// Field size Y in cm (0-40)
    pub field_size_y: f32,
    /// Dose rate in cGy/min
    pub dose_rate: f32,
}

impl Default for TreatmentParams {
    fn default() -> Self {
        Self {
            gantry_angle: 0,
            collimator_angle: 0,
            field_size_x: 10.0,
            field_size_y: 10.0,
            dose_rate: 100.0,
        }
    }
}

impl Default for Meos {
    fn default() -> Self {
        Self {
            beam_type: BeamType::default(),
            beam_energy: BeamEnergy::default(),
            collimator: CollimatorPosition::default(),
        }
    }
}

impl Meos {
    /// Check if the MEOS configuration is safe for treatment
    /// X-ray mode requires the flatness filter to be in position
    pub fn is_safe(&self) -> bool {
        match self.beam_type {
            BeamType::XRay => self.collimator == CollimatorPosition::InPosition,
            BeamType::Electron => self.collimator == CollimatorPosition::OutOfPosition,
            BeamType::Undefined => false,
        }
    }

    /// Check if collimator needs to move to match beam type
    pub fn needs_collimator_sync(&self) -> bool {
        !self.is_safe() && self.collimator != CollimatorPosition::Transitioning
    }
}

/// Treatment phase state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TPhase {
    /// System reset/initialization
    Reset,
    /// Data entry mode - operator entering treatment parameters
    DataEntry,
    /// Testing setup
    SetupTest,
    /// Setup complete, ready for treatment
    SetupDone,
    /// Patient treatment in progress
    PatientTreatment,
    /// Treatment paused (e.g., due to malfunction)
    PauseTreatment,
    /// Treatment terminated
    TerminateTreatment,
    /// Date/Time/ID changes
    DateTimeIdChanges,
}

impl Default for TPhase {
    fn default() -> Self {
        Self::Reset
    }
}

impl std::fmt::Display for TPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TPhase::Reset => write!(f, "Reset"),
            TPhase::DataEntry => write!(f, "Data Entry"),
            TPhase::SetupTest => write!(f, "Setup Test"),
            TPhase::SetupDone => write!(f, "Setup Done"),
            TPhase::PatientTreatment => write!(f, "Patient Treatment"),
            TPhase::PauseTreatment => write!(f, "Paused"),
            TPhase::TerminateTreatment => write!(f, "Terminated"),
            TPhase::DateTimeIdChanges => write!(f, "Date/Time/ID Changes"),
        }
    }
}

/// Main Therac-25 state structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheracState {
    /// Console MEOS - parameters entered by operator
    pub console_meos: Meos,
    /// Hardware MEOS - actual hardware configuration
    pub hardware_meos: Meos,
    /// Reference/prescribed MEOS - what the treatment plan specifies
    pub reference_meos: Meos,
    /// Console treatment parameters
    pub console_params: TreatmentParams,
    /// Hardware treatment parameters
    pub hardware_params: TreatmentParams,
    /// Reference treatment parameters
    pub reference_params: TreatmentParams,
    /// Current treatment phase
    pub phase: TPhase,
    /// Data entry complete flag
    pub data_entry_complete: bool,
    /// FSmall flag - set by collimator verification failure
    pub f_small: bool,
    /// Class3 counter - incremented during setup verification
    pub class3: u8,
    /// Bending magnet flag - indicates electron beam bending magnet status
    pub bending_magnet_flag: bool,
    /// Editing taking place - operator is modifying parameters
    pub editing_taking_place: bool,
    /// Reset pending - system reset has been requested
    pub reset_pending: bool,
    /// Class3 ignore - ignore Class3 verification
    pub class3_ignore: bool,
    /// Malfunction counter
    pub malfunction_count: u32,
    /// Total dose delivered (in cGy - centigray)
    pub dose_delivered: f64,
    /// Target dose (in cGy)
    pub dose_target: f64,
    /// Reference dose target (in cGy)
    pub reference_dose_target: f64,
    /// Treatment outcome message
    pub treatment_outcome: String,
    /// Treatment log
    pub log: Vec<String>,
    /// Last malfunction message
    pub last_malfunction: Option<String>,
}

impl Default for TheracState {
    fn default() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Generate random reference parameters (prescribed treatment)
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

        let collimator = match beam_type {
            BeamType::XRay => CollimatorPosition::InPosition,
            BeamType::Electron => CollimatorPosition::OutOfPosition,
            BeamType::Undefined => CollimatorPosition::OutOfPosition,
        };

        let reference_meos = Meos {
            beam_type,
            beam_energy,
            collimator,
        };

        let reference_dose = (rng.gen_range(150.0_f64..250.0_f64)).round();

        // Generate random reference treatment parameters
        let reference_params = TreatmentParams {
            gantry_angle: rng.gen_range(0..360),
            collimator_angle: rng.gen_range(0..360),
            field_size_x: rng.gen_range(5.0_f32..20.0_f32).round(),
            field_size_y: rng.gen_range(5.0_f32..20.0_f32).round(),
            dose_rate: match beam_type {
                BeamType::XRay => rng.gen_range(80.0_f32..120.0_f32).round(),
                BeamType::Electron => rng.gen_range(100.0_f32..200.0_f32).round(),
                BeamType::Undefined => 100.0,
            },
        };

        Self {
            console_meos: Meos::default(),
            hardware_meos: Meos::default(),
            reference_meos,
            console_params: TreatmentParams::default(),
            hardware_params: TreatmentParams::default(),
            reference_params,
            phase: TPhase::Reset,
            data_entry_complete: false,
            f_small: false,
            class3: 0,
            bending_magnet_flag: false,
            editing_taking_place: false,
            reset_pending: false,
            class3_ignore: false,
            malfunction_count: 0,
            dose_delivered: 0.0,
            dose_target: 200.0,
            reference_dose_target: reference_dose,
            treatment_outcome: String::new(),
            log: Vec::new(),
            last_malfunction: None,
        }
    }
}

impl TheracState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate new reference parameters (called on reset)
    pub fn generate_new_reference(&mut self) {
        use rand::Rng;
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

        let collimator = match beam_type {
            BeamType::XRay => CollimatorPosition::InPosition,
            BeamType::Electron => CollimatorPosition::OutOfPosition,
            BeamType::Undefined => CollimatorPosition::OutOfPosition,
        };

        self.reference_meos = Meos {
            beam_type,
            beam_energy,
            collimator,
        };

        self.reference_dose_target = (rng.gen_range(150.0_f64..250.0_f64)).round();

        self.reference_params = TreatmentParams {
            gantry_angle: rng.gen_range(0..360),
            collimator_angle: rng.gen_range(0..360),
            field_size_x: rng.gen_range(5.0_f32..20.0_f32).round(),
            field_size_y: rng.gen_range(5.0_f32..20.0_f32).round(),
            dose_rate: match beam_type {
                BeamType::XRay => rng.gen_range(80.0_f32..120.0_f32).round(),
                BeamType::Electron => rng.gen_range(100.0_f32..200.0_f32).round(),
                BeamType::Undefined => 100.0,
            },
        };

        self.add_log(format!(
            "New prescription: {} @ {} - {} cGy - Gantry {} deg - Field {}x{} cm",
            self.reference_meos.beam_type,
            self.reference_meos.beam_energy,
            self.reference_dose_target,
            self.reference_params.gantry_angle,
            self.reference_params.field_size_x,
            self.reference_params.field_size_y
        ));
    }

    pub fn add_log(&mut self, message: String) {
        self.log.push(format!("[{}] {}", chrono::Utc::now().format("%H:%M:%S"), message));
        // Keep only last 100 log entries
        if self.log.len() > 100 {
            self.log.drain(0..self.log.len() - 100);
        }
    }

    pub fn reset(&mut self) {
        self.phase = TPhase::Reset;
        self.data_entry_complete = false;
        self.f_small = false;
        self.class3 = 0;
        self.bending_magnet_flag = false;
        self.editing_taking_place = false;
        self.reset_pending = false;
        self.class3_ignore = false;
        self.dose_delivered = 0.0;
        self.dose_target = 200.0;
        self.last_malfunction = None;
        self.treatment_outcome = String::new();
        self.console_meos = Meos::default();
        self.console_params = TreatmentParams::default();
        self.add_log("System reset".to_string());
        self.generate_new_reference();
    }
}

/// Shared Therac state wrapped for concurrent access
pub type SharedTheracState = Arc<RwLock<TheracState>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meos_safety() {
        let xray_safe = Meos {
            beam_type: BeamType::XRay,
            beam_energy: BeamEnergy::E10,
            collimator: CollimatorPosition::InPosition,
        };
        assert!(xray_safe.is_safe());

        let xray_unsafe = Meos {
            beam_type: BeamType::XRay,
            beam_energy: BeamEnergy::E25,
            collimator: CollimatorPosition::OutOfPosition,
        };
        assert!(!xray_unsafe.is_safe());

        let electron_safe = Meos {
            beam_type: BeamType::Electron,
            beam_energy: BeamEnergy::E15,
            collimator: CollimatorPosition::OutOfPosition,
        };
        assert!(electron_safe.is_safe());
    }

    #[test]
    fn test_collimator_sync_needed() {
        let needs_sync = Meos {
            beam_type: BeamType::XRay,
            beam_energy: BeamEnergy::E10,
            collimator: CollimatorPosition::OutOfPosition,
        };
        assert!(needs_sync.needs_collimator_sync());
    }
}
