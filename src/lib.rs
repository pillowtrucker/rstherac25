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
pub mod tui;

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
    /// Current treatment phase
    pub phase: TPhase,
    /// Data entry complete flag
    pub data_entry_complete: bool,
    /// FSmall flag - set by collimator verification failure
    pub f_small: bool,
    /// Class3 counter - incremented during setup verification
    pub class3: u8,
    /// Malfunction counter
    pub malfunction_count: u32,
    /// Total dose delivered (in cGy - centigray)
    pub dose_delivered: f64,
    /// Target dose (in cGy)
    pub dose_target: f64,
    /// Treatment log
    pub log: Vec<String>,
    /// Last malfunction message
    pub last_malfunction: Option<String>,
}

impl Default for TheracState {
    fn default() -> Self {
        Self {
            console_meos: Meos::default(),
            hardware_meos: Meos::default(),
            phase: TPhase::Reset,
            data_entry_complete: false,
            f_small: false,
            class3: 0,
            malfunction_count: 0,
            dose_delivered: 0.0,
            dose_target: 200.0, // Default 200 cGy
            log: Vec::new(),
            last_malfunction: None,
        }
    }
}

impl TheracState {
    pub fn new() -> Self {
        Self::default()
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
        self.dose_delivered = 0.0;
        self.last_malfunction = None;
        self.add_log("System reset".to_string());
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
