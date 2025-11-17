//! WebAssembly bindings for Therac-25 simulator
//!
//! This module provides a browser-based interface for the simulator

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;
use crate::*;
use crate::simulator::*;
use std::sync::Arc;
use parking_lot::RwLock;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// WebAssembly interface for Therac-25 simulator
#[wasm_bindgen]
pub struct WasmTherac25 {
    state: SharedTheracState,
}

#[wasm_bindgen]
impl WasmTherac25 {
    /// Create new simulator instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmTherac25, JsValue> {
        // Set panic hook for better error messages
        console_error_panic_hook::set_once();

        console::log_1(&"Initializing Therac-25 simulator...".into());

        let state = Arc::new(RwLock::new(TheracState::new()));
        state.write().add_log("System initialized".to_string());

        // Start concurrent tasks
        let state_clone1 = state.clone();
        let state_clone2 = state.clone();

        spawn_local(async move {
            treatment_monitor(state_clone1).await;
        });

        spawn_local(async move {
            housekeeper(state_clone2).await;
        });

        Ok(WasmTherac25 { state })
    }

    /// Get current state as JSON
    #[wasm_bindgen(js_name = getState)]
    pub fn get_state(&self) -> JsValue {
        let state = self.state.read();
        serde_wasm_bindgen::to_value(&*state).unwrap_or(JsValue::NULL)
    }

    /// Reset the system
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        let mut state = self.state.write();
        state.reset();
    }

    /// Set beam type (0 = XRay, 1 = Electron, 2 = Undefined)
    #[wasm_bindgen(js_name = setBeamType)]
    pub fn set_beam_type(&mut self, beam_type: u8) {
        let mut state = self.state.write();
        if state.phase == TPhase::DataEntry {
            state.console_meos.beam_type = match beam_type {
                0 => BeamType::XRay,
                1 => BeamType::Electron,
                _ => BeamType::Undefined,
            };
            let beam_type_val = state.console_meos.beam_type;
            state.add_log(format!("Beam type set to {}", beam_type_val));
        }
    }

    /// Set beam energy (0-4 for E5-E25)
    #[wasm_bindgen(js_name = setBeamEnergy)]
    pub fn set_beam_energy(&mut self, energy: u8) {
        let mut state = self.state.write();
        if state.phase == TPhase::DataEntry {
            state.console_meos.beam_energy = match energy {
                0 => BeamEnergy::E5,
                1 => BeamEnergy::E10,
                2 => BeamEnergy::E15,
                3 => BeamEnergy::E20,
                _ => BeamEnergy::E25,
            };
            let beam_energy_val = state.console_meos.beam_energy;
            state.add_log(format!("Beam energy set to {}", beam_energy_val));
        }
    }

    /// Toggle collimator position
    #[wasm_bindgen(js_name = toggleCollimator)]
    pub fn toggle_collimator(&mut self) {
        let mut state = self.state.write();
        state.console_meos.collimator = match state.console_meos.collimator {
            CollimatorPosition::InPosition => CollimatorPosition::OutOfPosition,
            CollimatorPosition::OutOfPosition => CollimatorPosition::InPosition,
            CollimatorPosition::Transitioning => CollimatorPosition::InPosition,
        };
        let collimator_val = state.console_meos.collimator;
        state.add_log(format!("Collimator set to {}", collimator_val));
    }

    /// Complete data entry
    #[wasm_bindgen(js_name = completeDataEntry)]
    pub fn complete_data_entry(&mut self) {
        complete_data_entry(self.state.clone());
    }

    /// Start treatment
    #[wasm_bindgen(js_name = startTreatment)]
    pub fn start_treatment(&mut self) {
        start_treatment(self.state.clone());
    }

    /// Stop treatment
    #[wasm_bindgen(js_name = stopTreatment)]
    pub fn stop_treatment(&mut self) {
        stop_treatment(self.state.clone());
    }

    /// Resume treatment
    #[wasm_bindgen(js_name = resumeTreatment)]
    pub fn resume_treatment(&mut self) {
        resume_treatment(self.state.clone());
    }

    /// Generate random safe parameters
    #[wasm_bindgen(js_name = generateRandomParameters)]
    pub fn generate_random_parameters(&mut self) {
        let params = generate_random_parameters();
        update_console_meos(self.state.clone(), params);
    }

    /// Generate race condition parameters
    #[wasm_bindgen(js_name = generateRaceConditionParameters)]
    pub fn generate_race_condition_parameters(&mut self) {
        let current = self.state.read().console_meos;
        let params = generate_race_condition_parameters(current);
        update_console_meos(self.state.clone(), params);
    }

    /// Get log messages
    #[wasm_bindgen(js_name = getLog)]
    pub fn get_log(&self) -> Vec<JsValue> {
        let state = self.state.read();
        state.log.iter()
            .rev()
            .take(50)
            .map(|s| JsValue::from_str(s))
            .collect()
    }

    /// Get current phase as string
    #[wasm_bindgen(js_name = getPhase)]
    pub fn get_phase(&self) -> String {
        let state = self.state.read();
        format!("{}", state.phase)
    }

    /// Check if hardware is safe
    #[wasm_bindgen(js_name = isSafe)]
    pub fn is_safe(&self) -> bool {
        let state = self.state.read();
        state.hardware_meos.is_safe()
    }

    /// Get dose delivered
    #[wasm_bindgen(js_name = getDoseDelivered)]
    pub fn get_dose_delivered(&self) -> f64 {
        let state = self.state.read();
        state.dose_delivered
    }

    /// Get dose target
    #[wasm_bindgen(js_name = getDoseTarget)]
    pub fn get_dose_target(&self) -> f64 {
        let state = self.state.read();
        state.dose_target
    }

    /// Set dose target
    #[wasm_bindgen(js_name = setDoseTarget)]
    pub fn set_dose_target(&mut self, target: f64) {
        let mut state = self.state.write();
        state.dose_target = target;
        state.add_log(format!("Dose target set to {:.1} cGy", target));
    }

    /// Set gantry angle (0-360 degrees)
    #[wasm_bindgen(js_name = setGantryAngle)]
    pub fn set_gantry_angle(&mut self, angle: u16) {
        let mut state = self.state.write();
        if state.phase == TPhase::DataEntry {
            state.console_params.gantry_angle = angle.min(360);
            let angle_val = state.console_params.gantry_angle;
            state.add_log(format!("Gantry angle set to {}°", angle_val));
        }
    }

    /// Set field size X dimension (cm)
    #[wasm_bindgen(js_name = setFieldSizeX)]
    pub fn set_field_size_x(&mut self, size: f32) {
        let mut state = self.state.write();
        if state.phase == TPhase::DataEntry {
            state.console_params.field_size_x = size.max(0.0).min(40.0);
            let size_val = state.console_params.field_size_x;
            state.add_log(format!("Field size X set to {:.1} cm", size_val));
        }
    }

    /// Set field size Y dimension (cm)
    #[wasm_bindgen(js_name = setFieldSizeY)]
    pub fn set_field_size_y(&mut self, size: f32) {
        let mut state = self.state.write();
        if state.phase == TPhase::DataEntry {
            state.console_params.field_size_y = size.max(0.0).min(40.0);
            let size_val = state.console_params.field_size_y;
            state.add_log(format!("Field size Y set to {:.1} cm", size_val));
        }
    }

    /// Set dose rate (cGy/min)
    #[wasm_bindgen(js_name = setDoseRate)]
    pub fn set_dose_rate(&mut self, rate: f32) {
        let mut state = self.state.write();
        if state.phase == TPhase::DataEntry {
            state.console_params.dose_rate = rate.max(0.0);
            let rate_val = state.console_params.dose_rate;
            state.add_log(format!("Dose rate set to {:.0} cGy/min", rate_val));
        }
    }

    /// Get gantry angle
    #[wasm_bindgen(js_name = getGantryAngle)]
    pub fn get_gantry_angle(&self) -> u16 {
        let state = self.state.read();
        state.console_params.gantry_angle
    }

    /// Get field size X
    #[wasm_bindgen(js_name = getFieldSizeX)]
    pub fn get_field_size_x(&self) -> f32 {
        let state = self.state.read();
        state.console_params.field_size_x
    }

    /// Get field size Y
    #[wasm_bindgen(js_name = getFieldSizeY)]
    pub fn get_field_size_y(&self) -> f32 {
        let state = self.state.read();
        state.console_params.field_size_y
    }

    /// Get dose rate
    #[wasm_bindgen(js_name = getDoseRate)]
    pub fn get_dose_rate(&self) -> f32 {
        let state = self.state.read();
        state.console_params.dose_rate
    }

    /// Get reference (prescription) parameters as JSON
    #[wasm_bindgen(js_name = getReferenceParams)]
    pub fn get_reference_params(&self) -> JsValue {
        let state = self.state.read();
        let params = serde_json::json!({
            "beam_type": format!("{}", state.reference_meos.beam_type),
            "beam_energy": format!("{}", state.reference_meos.beam_energy),
            "gantry_angle": state.reference_params.gantry_angle,
            "field_size_x": state.reference_params.field_size_x,
            "field_size_y": state.reference_params.field_size_y,
            "dose_rate": state.reference_params.dose_rate,
            "dose_target": state.reference_dose_target,
        });
        serde_wasm_bindgen::to_value(&params).unwrap_or(JsValue::NULL)
    }

    /// Get malfunction count
    #[wasm_bindgen(js_name = getMalfunctionCount)]
    pub fn get_malfunction_count(&self) -> u32 {
        let state = self.state.read();
        state.malfunction_count
    }

    /// Get last malfunction message
    #[wasm_bindgen(js_name = getLastMalfunction)]
    pub fn get_last_malfunction(&self) -> Option<String> {
        let state = self.state.read();
        state.last_malfunction.clone()
    }
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    console::log_1(&"Therac-25 WASM module loaded".into());
}
