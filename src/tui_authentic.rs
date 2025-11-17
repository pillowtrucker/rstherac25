//! Authentic Therac-25 VT100 Terminal Interface
//!
//! This module recreates the original Therac-25 operator console interface
//! as it appeared on the DEC VT100 terminal in the 1980s.

use crate::*;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Clear, Paragraph},
    Frame, Terminal,
};
use std::io;

// Column positions matching original Therac-25 layout
const LEFT: usize = 10;
const CENTER_LEFT: usize = 33;
const CENTER_RIGHT: usize = 50;
const RIGHT: usize = 70;

/// Input field positions on the authentic interface
#[derive(Debug, Clone, Copy, PartialEq)]
enum InputField {
    PatientName,
    Mode,          // X or E
    Energy,        // KeV
    UnitRate,      // Prescribed
    MonitorUnits,  // Prescribed
    Time,          // Prescribed
    GantryRot,     // Prescribed
    CollimatorRot, // Prescribed
    CollimatorX,   // Prescribed
    CollimatorY,   // Prescribed
    WedgeNum,      // Prescribed
    AccessoryNum,  // Prescribed
    Command,       // Bottom command line
}

pub struct AuthenticTuiApp {
    state: SharedTheracState,
    current_field: InputField,

    // Input buffers
    patient_name: String,
    mode_input: String,
    energy_input: String,
    unit_rate_input: String,
    monitor_units_input: String,
    time_input: String,
    gantry_rot_input: String,
    collimator_rot_input: String,
    collimator_x_input: String,
    collimator_y_input: String,
    wedge_num_input: String,
    accessory_num_input: String,
    command_input: String,

    // Malfunction popup
    show_malfunction: bool,
    malfunction_message: String,
}

impl AuthenticTuiApp {
    pub fn new(state: SharedTheracState) -> Self {
        // Initialize with a random prescription and sync hardware to match
        {
            let mut s = state.write();
            s.generate_new_reference();
            // Initialize hardware to match the reference prescription
            s.hardware_meos = s.reference_meos;
            s.hardware_params = s.reference_params;
        }

        Self {
            state,
            current_field: InputField::PatientName,
            patient_name: String::new(),
            mode_input: String::new(),
            energy_input: String::new(),
            unit_rate_input: String::new(),
            monitor_units_input: String::new(),
            time_input: String::new(),
            gantry_rot_input: String::new(),
            collimator_rot_input: String::new(),
            collimator_x_input: String::new(),
            collimator_y_input: String::new(),
            wedge_num_input: String::new(),
            accessory_num_input: String::new(),
            command_input: String::new(),
            show_malfunction: false,
            malfunction_message: String::new(),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q')
                        if self.current_field == InputField::Command => {
                        break;
                    }
                    KeyCode::Esc => {
                        if self.show_malfunction {
                            self.show_malfunction = false;
                        } else {
                            break;
                        }
                    }
                    _ => {
                        if self.show_malfunction {
                            // Any key dismisses malfunction popup
                            self.show_malfunction = false;
                        } else {
                            self.handle_input(key.code);
                        }
                    }
                }
            }

            // Check for malfunctions
            {
                let s = self.state.read();
                if let Some(ref msg) = s.last_malfunction {
                    if !self.show_malfunction {
                        self.malfunction_message = msg.clone();
                        self.show_malfunction = true;
                    }
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Down => self.next_field(),
            KeyCode::Up => self.prev_field(),
            KeyCode::Tab => self.next_field(),
            KeyCode::BackTab => self.prev_field(),
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Char(c) => self.handle_char(c),
            _ => {}
        }
    }

    fn next_field(&mut self) {
        self.current_field = match self.current_field {
            InputField::PatientName => InputField::Mode,
            InputField::Mode => InputField::Energy,
            InputField::Energy => InputField::UnitRate,
            InputField::UnitRate => InputField::MonitorUnits,
            InputField::MonitorUnits => InputField::Time,
            InputField::Time => InputField::GantryRot,
            InputField::GantryRot => InputField::CollimatorRot,
            InputField::CollimatorRot => InputField::CollimatorX,
            InputField::CollimatorX => InputField::CollimatorY,
            InputField::CollimatorY => InputField::WedgeNum,
            InputField::WedgeNum => InputField::AccessoryNum,
            InputField::AccessoryNum => InputField::Command,
            InputField::Command => InputField::PatientName,
        };
    }

    fn prev_field(&mut self) {
        self.current_field = match self.current_field {
            InputField::PatientName => InputField::Command,
            InputField::Mode => InputField::PatientName,
            InputField::Energy => InputField::Mode,
            InputField::UnitRate => InputField::Energy,
            InputField::MonitorUnits => InputField::UnitRate,
            InputField::Time => InputField::MonitorUnits,
            InputField::GantryRot => InputField::Time,
            InputField::CollimatorRot => InputField::GantryRot,
            InputField::CollimatorX => InputField::CollimatorRot,
            InputField::CollimatorY => InputField::CollimatorX,
            InputField::WedgeNum => InputField::CollimatorY,
            InputField::AccessoryNum => InputField::WedgeNum,
            InputField::Command => InputField::AccessoryNum,
        };
    }

    fn handle_enter(&mut self) {
        match self.current_field {
            InputField::Mode => {
                let c = self.mode_input.to_uppercase();
                if c == "X" {
                    let mut s = self.state.write();
                    s.console_meos.beam_type = BeamType::XRay;
                    s.console_meos.beam_energy = BeamEnergy::E25;
                    self.energy_input = "25000".to_string(); // 25 MeV = 25000 KeV
                    s.add_log("Mode: X-Ray, Energy: 25 MeV".to_string());
                } else if c == "E" {
                    let mut s = self.state.write();
                    s.console_meos.beam_type = BeamType::Electron;
                    s.add_log("Mode: Electron".to_string());
                }
                self.next_field();
            }
            InputField::Energy => {
                // Auto-copy if empty
                if self.energy_input.is_empty() {
                    let s = self.state.read();
                    let energy_mev = match s.reference_meos.beam_energy {
                        BeamEnergy::E5 => 5,
                        BeamEnergy::E10 => 10,
                        BeamEnergy::E15 => 15,
                        BeamEnergy::E20 => 20,
                        BeamEnergy::E25 => 25,
                    };
                    self.energy_input = (energy_mev * 1000).to_string();
                }
                self.next_field();
            }
            InputField::UnitRate => {
                // Auto-copy if empty
                if self.unit_rate_input.is_empty() {
                    let s = self.state.read();
                    self.unit_rate_input = format!("{:.1}", s.reference_params.dose_rate);
                }
                self.next_field();
            }
            InputField::MonitorUnits => {
                // Auto-copy if empty (not implemented, skip)
                self.next_field();
            }
            InputField::Time => {
                // Auto-copy if empty - calculate from dose target and rate
                if self.time_input.is_empty() {
                    let s = self.state.read();
                    if s.reference_params.dose_rate > 0.0 {
                        let time = s.reference_dose_target / (s.reference_params.dose_rate as f64);
                        self.time_input = format!("{:.1}", time);
                    }
                }
                self.next_field();
            }
            InputField::GantryRot => {
                // Auto-copy if empty
                if self.gantry_rot_input.is_empty() {
                    let s = self.state.read();
                    self.gantry_rot_input = s.reference_params.gantry_angle.to_string();
                }
                self.next_field();
            }
            InputField::CollimatorRot => {
                // Auto-copy if empty
                if self.collimator_rot_input.is_empty() {
                    let s = self.state.read();
                    self.collimator_rot_input = s.reference_params.collimator_angle.to_string();
                }
                self.next_field();
            }
            InputField::CollimatorX => {
                // Auto-copy if empty
                if self.collimator_x_input.is_empty() {
                    let s = self.state.read();
                    self.collimator_x_input = format!("{:.1}", s.reference_params.field_size_x);
                }
                self.next_field();
            }
            InputField::CollimatorY => {
                // Auto-copy if empty
                if self.collimator_y_input.is_empty() {
                    let s = self.state.read();
                    self.collimator_y_input = format!("{:.1}", s.reference_params.field_size_y);
                }
                self.next_field();
            }
            InputField::WedgeNum => {
                // Auto-copy if empty (not implemented, use 0)
                if self.wedge_num_input.is_empty() {
                    self.wedge_num_input = "0".to_string();
                }
                self.next_field();
            }
            InputField::AccessoryNum => {
                // Auto-copy if empty (not implemented, use 0)
                if self.accessory_num_input.is_empty() {
                    self.accessory_num_input = "0".to_string();
                }
                self.next_field();
            }
            InputField::Command => {
                self.handle_command();
            }
            InputField::PatientName => {
                self.next_field();
            }
        }
    }

    fn handle_backspace(&mut self) {
        match self.current_field {
            InputField::PatientName => { self.patient_name.pop(); }
            InputField::Mode => { self.mode_input.pop(); }
            InputField::Energy => { self.energy_input.pop(); }
            InputField::UnitRate => { self.unit_rate_input.pop(); }
            InputField::MonitorUnits => { self.monitor_units_input.pop(); }
            InputField::Time => { self.time_input.pop(); }
            InputField::GantryRot => { self.gantry_rot_input.pop(); }
            InputField::CollimatorRot => { self.collimator_rot_input.pop(); }
            InputField::CollimatorX => { self.collimator_x_input.pop(); }
            InputField::CollimatorY => { self.collimator_y_input.pop(); }
            InputField::WedgeNum => { self.wedge_num_input.pop(); }
            InputField::AccessoryNum => { self.accessory_num_input.pop(); }
            InputField::Command => { self.command_input.pop(); }
        }
    }

    fn handle_char(&mut self, c: char) {
        match self.current_field {
            InputField::PatientName if c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == ',' => {
                self.patient_name.push(c);
            }
            InputField::Mode if (c == 'X' || c == 'x' || c == 'E' || c == 'e') && self.mode_input.is_empty() => {
                // Only allow X or E, and only one character
                self.mode_input.push(c.to_ascii_uppercase());
            }
            InputField::Energy if c.is_ascii_digit() => self.energy_input.push(c),
            InputField::UnitRate if c.is_ascii_digit() || c == '.' => self.unit_rate_input.push(c),
            InputField::MonitorUnits if c.is_ascii_digit() || c == '.' => self.monitor_units_input.push(c),
            InputField::Time if c.is_ascii_digit() || c == '.' => self.time_input.push(c),
            InputField::GantryRot if c.is_ascii_digit() => self.gantry_rot_input.push(c),
            InputField::CollimatorRot if c.is_ascii_digit() => self.collimator_rot_input.push(c),
            InputField::CollimatorX if c.is_ascii_digit() || c == '.' => self.collimator_x_input.push(c),
            InputField::CollimatorY if c.is_ascii_digit() || c == '.' => self.collimator_y_input.push(c),
            InputField::WedgeNum if c.is_ascii_digit() => self.wedge_num_input.push(c),
            InputField::AccessoryNum if c.is_ascii_digit() => self.accessory_num_input.push(c),
            InputField::Command if c.is_alphanumeric() || c.is_whitespace() => self.command_input.push(c),
            _ => {}
        }
    }

    fn handle_command(&mut self) {
        let cmd = self.command_input.trim().to_lowercase();
        match cmd.as_str() {
            "p" | "proceed" => {
                // Set prescribed values from inputs
                self.apply_prescription();
                crate::simulator::complete_data_entry(self.state.clone());
                self.command_input.clear();
            }
            "t" | "treat" => {
                self.apply_prescription();
                crate::simulator::complete_data_entry(self.state.clone());
                crate::simulator::start_treatment(self.state.clone());
                self.command_input.clear();
            }
            "r" | "reset" => {
                let mut s = self.state.write();
                s.reset();
                s.generate_new_reference();
                // Sync hardware to match the new reference prescription
                s.hardware_meos = s.reference_meos;
                s.hardware_params = s.reference_params;
                drop(s);
                self.clear_all_inputs();
                self.command_input.clear();
            }
            "s" | "stop" => {
                crate::simulator::stop_treatment(self.state.clone());
                self.command_input.clear();
            }
            "c" | "continue" => {
                crate::simulator::resume_treatment(self.state.clone());
                self.command_input.clear();
            }
            _ => {}
        }
    }

    fn apply_prescription(&mut self) {
        let mut s = self.state.write();

        // Parse and apply all prescribed values
        if let Ok(rate) = self.unit_rate_input.parse::<f32>() {
            s.console_params.dose_rate = rate;
        }
        if let Ok(gantry) = self.gantry_rot_input.parse::<u16>() {
            s.console_params.gantry_angle = gantry;
        }
        if let Ok(coll_rot) = self.collimator_rot_input.parse::<u16>() {
            s.console_params.collimator_angle = coll_rot;
        }
        if let Ok(coll_x) = self.collimator_x_input.parse::<f32>() {
            s.console_params.field_size_x = coll_x;
        }
        if let Ok(coll_y) = self.collimator_y_input.parse::<f32>() {
            s.console_params.field_size_y = coll_y;
        }
        if let Ok(time) = self.time_input.parse::<f32>() {
            // Calculate dose from time and rate
            if let Ok(rate) = self.unit_rate_input.parse::<f32>() {
                s.dose_target = (time * rate) as f64;
            }
        }
        if let Ok(energy_kev) = self.energy_input.parse::<u32>() {
            // Convert KeV to MeV and set energy
            let energy_mev = energy_kev / 1000;
            s.console_meos.beam_energy = match energy_mev {
                5 => BeamEnergy::E5,
                10 => BeamEnergy::E10,
                15 => BeamEnergy::E15,
                20 => BeamEnergy::E20,
                25 => BeamEnergy::E25,
                _ => s.console_meos.beam_energy,
            };
        }

        s.editing_taking_place = true;
    }

    fn clear_all_inputs(&mut self) {
        self.patient_name.clear();
        self.mode_input.clear();
        self.energy_input.clear();
        self.unit_rate_input.clear();
        self.monitor_units_input.clear();
        self.time_input.clear();
        self.gantry_rot_input.clear();
        self.collimator_rot_input.clear();
        self.collimator_x_input.clear();
        self.collimator_y_input.clear();
        self.wedge_num_input.clear();
        self.accessory_num_input.clear();
    }

    fn render(&self, f: &mut Frame) {
        if self.show_malfunction {
            self.render_main_screen(f);
            self.render_malfunction_popup(f);
        } else {
            self.render_main_screen(f);
        }
    }

    fn render_main_screen(&self, f: &mut Frame) {
        let state = self.state.read();

        // VT100-style monochrome interface with green text on black
        let area = f.area();

        let block = Block::default()
            .style(Style::default().bg(Color::Black).fg(Color::Green));
        f.render_widget(block, area);

        // Build the screen content
        let mut lines = vec![];

        // Line 1: Patient name
        lines.push(self.render_field_line(
            "Patient Name:",
            &self.patient_name,
            self.current_field == InputField::PatientName,
        ));

        // Line 2: Blank
        lines.push(Line::from(""));

        // Line 3: Mode and Energy
        // Display full mode name based on input
        let mode_display = if self.mode_input.is_empty() {
            String::new()
        } else if self.mode_input == "X" {
            "X-RAY".to_string()
        } else if self.mode_input == "E" {
            "ELECTRON".to_string()
        } else {
            self.mode_input.clone()
        };

        lines.push(Line::from(vec![
            Span::raw(format!("{:>LEFT$}Mode: ", "")),
            Span::styled(
                format!("{:<10}", mode_display),
                if self.current_field == InputField::Mode {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::Green)
                }
            ),
            if self.current_field == InputField::Mode {
                Span::styled(" ◀", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("")
            },
            Span::raw("    Energy (KeV): "),
            Span::styled(
                format!("{:<10}", self.energy_input),
                if self.current_field == InputField::Energy {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::Green)
                }
            ),
            if self.current_field == InputField::Energy {
                Span::styled(" ◀", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("")
            },
        ]));

        // Line 4: Blank
        lines.push(Line::from(""));

        // Line 5: Header for parameters
        lines.push(Line::from(vec![
            Span::raw(format!("{:>CENTER_LEFT$}ACTUAL", "")),
            Span::raw(format!("{:>17}PRESCRIBED", "")),
        ]));

        // Lines 6-15: Parameter rows
        lines.push(self.render_param_line("Unit rate/min:",
            &format!("{:.1}", state.hardware_params.dose_rate),
            &self.unit_rate_input,
            self.current_field == InputField::UnitRate));

        lines.push(self.render_param_line("Monitor units:",
            "0", // Not implemented in our simulator
            &self.monitor_units_input,
            self.current_field == InputField::MonitorUnits));

        lines.push(self.render_param_line("Time (minutes):",
            &format!("{:.1}", if state.dose_delivered > 0.0 { state.dose_delivered / (state.hardware_params.dose_rate as f64) } else { 0.0 }),
            &self.time_input,
            self.current_field == InputField::Time));

        lines.push(self.render_param_line("Gantry rotation (deg):",
            &format!("{}", state.hardware_params.gantry_angle),
            &self.gantry_rot_input,
            self.current_field == InputField::GantryRot));

        lines.push(self.render_param_line("Collimator rotation (deg):",
            &format!("{}", state.hardware_params.collimator_angle),
            &self.collimator_rot_input,
            self.current_field == InputField::CollimatorRot));

        lines.push(self.render_param_line("Collimator x (cm):",
            &format!("{:.1}", state.hardware_params.field_size_x),
            &self.collimator_x_input,
            self.current_field == InputField::CollimatorX));

        lines.push(self.render_param_line("Collimator y (cm):",
            &format!("{:.1}", state.hardware_params.field_size_y),
            &self.collimator_y_input,
            self.current_field == InputField::CollimatorY));

        lines.push(self.render_param_line("Wedge number:",
            "0",
            &self.wedge_num_input,
            self.current_field == InputField::WedgeNum));

        lines.push(self.render_param_line("Accessory number:",
            "0",
            &self.accessory_num_input,
            self.current_field == InputField::AccessoryNum));

        // Line 16: Blank
        lines.push(Line::from(""));

        // Line 17: Verification status
        let verified = self.check_verification(&state);
        lines.push(Line::from(vec![
            Span::raw(format!("{:>RIGHT$}", "")),
            Span::styled(
                if verified { "VERIFIED" } else { "" },
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            ),
        ]));

        // Line 18-19: Blank
        lines.push(Line::from(""));
        lines.push(Line::from(""));

        // Line 20: Command line
        lines.push(Line::from(vec![
            Span::raw("Command: "),
            Span::styled(
                format!("{:<30}", &self.command_input),
                if self.current_field == InputField::Command {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::Green)
                }
            ),
            if self.current_field == InputField::Command {
                Span::styled(" ◀", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("")
            },
        ]));

        // Line 21: Status
        let phase_msg = format!("Phase: {:?}  |  Safety: {}",
            state.phase,
            if state.hardware_meos.is_safe() { "SAFE" } else { "UNSAFE" }
        );
        lines.push(Line::from(Span::styled(
            phase_msg,
            Style::default().fg(Color::DarkGray)
        )));

        let paragraph = Paragraph::new(lines)
            .style(Style::default().bg(Color::Black).fg(Color::Green));

        f.render_widget(paragraph, area);
    }

    fn render_field_line(&self, label: &str, value: &str, active: bool) -> Line {
        Line::from(vec![
            Span::raw(format!("{:>LEFT$}{} ", "", label)),
            Span::styled(
                format!("{:<40}", value),
                if active {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::Green)
                }
            ),
            if active {
                Span::styled(" ◀", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("")
            },
        ])
    }

    fn render_param_line(&self, label: &str, actual: &str, prescribed: &str, active: bool) -> Line {
        Line::from(vec![
            Span::raw(format!("{:>LEFT$}{:<20}", "", label)),
            Span::raw(format!("{:>13}", actual)),
            Span::raw("      "),
            Span::styled(
                format!("{:<10}", prescribed),
                if active {
                    Style::default().fg(Color::Black).bg(Color::Green)
                } else {
                    Style::default().fg(Color::Green)
                }
            ),
            if active {
                Span::styled(" ◀", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("")
            },
        ])
    }

    fn check_verification(&self, _state: &TheracState) -> bool {
        // In the real Therac-25, this checked if actual == prescribed
        // For our simulator, we'll show VERIFIED when data entry is complete
        !self.unit_rate_input.is_empty() &&
        !self.gantry_rot_input.is_empty() &&
        !self.collimator_x_input.is_empty() &&
        !self.collimator_y_input.is_empty()
    }

    fn render_malfunction_popup(&self, f: &mut Frame) {
        let area = centered_rect(60, 40, f.area());

        // Parse malfunction number if present
        let malfunction_num = if self.malfunction_message.contains("MALFUNCTION") {
            if self.malfunction_message.contains("54") {
                "54"
            } else if self.malfunction_message.contains("CRITICAL") {
                "26"
            } else {
                "13"
            }
        } else {
            "54"
        };

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("MALFUNCTION {}", malfunction_num),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            )),
            Line::from(""),
            Line::from(Span::styled(
                &self.malfunction_message,
                Style::default().fg(Color::Yellow)
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to continue",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
            )),
        ];

        let block = Paragraph::new(text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Red)))
            .alignment(Alignment::Center)
            .style(Style::default().bg(Color::Black));

        f.render_widget(Clear, area);
        f.render_widget(block, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
