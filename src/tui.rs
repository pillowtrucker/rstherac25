//! Terminal User Interface for Therac-25 simulator
//!
//! Provides an accurate recreation of the Therac-25 operator interface with
//! form-based data entry and command input

use crate::*;
use crate::simulator::*;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Paragraph, List, ListItem, Gauge},
    Frame, Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputField {
    Mode,
    Energy,
    Gantry,
    FieldSize,
    Dose,
    Command,
}

pub struct TuiApp {
    state: SharedTheracState,
    should_quit: bool,
    help_visible: bool,
    current_field: InputField,
    mode_input: String,
    energy_input: String,
    gantry_input: String,
    field_x_input: String,
    field_y_input: String,
    dose_input: String,
    command_input: String,
}

impl TuiApp {
    pub fn new(state: SharedTheracState) -> Self {
        Self {
            state,
            should_quit: false,
            help_visible: false,
            current_field: InputField::Mode,
            mode_input: String::new(),
            energy_input: String::new(),
            gantry_input: String::new(),
            field_x_input: String::new(),
            field_y_input: String::new(),
            dose_input: String::new(),
            command_input: String::new(),
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Initialize with data entry mode
        {
            let mut s = self.state.write();
            s.phase = TPhase::DataEntry;
        }

        // Run the TUI loop
        let result = self.tui_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn tui_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if self.should_quit {
                break;
            }

            // Poll for events with timeout
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_input(key.code, key.modifiers);
                }
            }
        }

        Ok(())
    }

    fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if self.help_visible {
            self.help_visible = false;
            return;
        }

        // Global commands
        match key {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            KeyCode::F(1) => {
                self.help_visible = true;
                return;
            }
            _ => {}
        }

        // Handle input based on current field
        match self.current_field {
            InputField::Mode => self.handle_mode_input(key),
            InputField::Energy => self.handle_energy_input(key),
            InputField::Gantry => self.handle_gantry_input(key),
            InputField::FieldSize => self.handle_field_size_input(key),
            InputField::Dose => self.handle_dose_input(key),
            InputField::Command => self.handle_command_input(key),
        }
    }

    fn handle_mode_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.mode_input = "X".to_string();
                let mut s = self.state.write();
                s.console_meos.beam_type = BeamType::XRay;
                // Auto-set energy to 25 MeV for X-ray mode (as per real Therac-25)
                s.console_meos.beam_energy = BeamEnergy::E25;
                self.energy_input = "25".to_string();
                s.add_log("Mode set to X-Ray, energy auto-set to 25 MeV".to_string());
                // Move to gantry field (skip energy since it's auto-set)
                self.current_field = InputField::Gantry;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.mode_input = "E".to_string();
                let mut s = self.state.write();
                s.console_meos.beam_type = BeamType::Electron;
                s.add_log("Mode set to Electron".to_string());
                // Move to energy field
                self.current_field = InputField::Energy;
            }
            KeyCode::Enter => {
                // Copy from reference
                let s = self.state.read();
                match s.reference_meos.beam_type {
                    BeamType::XRay => {
                        drop(s);
                        self.handle_mode_input(KeyCode::Char('x'));
                    }
                    BeamType::Electron => {
                        drop(s);
                        self.handle_mode_input(KeyCode::Char('e'));
                    }
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                self.mode_input.clear();
            }
            _ => {}
        }
    }

    fn handle_energy_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.energy_input.push(c);
            }
            KeyCode::Backspace => {
                self.energy_input.pop();
            }
            KeyCode::Enter => {
                if self.energy_input.is_empty() {
                    // Copy from reference
                    let s = self.state.read();
                    let ref_energy = match s.reference_meos.beam_energy {
                        BeamEnergy::E5 => 5,
                        BeamEnergy::E10 => 10,
                        BeamEnergy::E15 => 15,
                        BeamEnergy::E20 => 20,
                        BeamEnergy::E25 => 25,
                    };
                    self.energy_input = ref_energy.to_string();
                }

                // Parse and set energy
                if let Ok(energy_val) = self.energy_input.parse::<u8>() {
                    let mut s = self.state.write();
                    s.console_meos.beam_energy = match energy_val {
                        5 => BeamEnergy::E5,
                        10 => BeamEnergy::E10,
                        15 => BeamEnergy::E15,
                        20 => BeamEnergy::E20,
                        25 => BeamEnergy::E25,
                        _ => {
                            s.add_log(format!("Invalid energy: {}. Use 5, 10, 15, 20, or 25", energy_val));
                            return;
                        }
                    };
                    s.add_log(format!("Energy set to {} MeV", energy_val));
                }
                // Move to gantry field
                self.current_field = InputField::Gantry;
            }
            _ => {}
        }
    }

    fn handle_gantry_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.gantry_input.push(c);
            }
            KeyCode::Backspace => {
                self.gantry_input.pop();
            }
            KeyCode::Enter => {
                if self.gantry_input.is_empty() {
                    // Copy from reference
                    let s = self.state.read();
                    self.gantry_input = s.reference_params.gantry_angle.to_string();
                }

                // Parse and set gantry angle
                if let Ok(angle) = self.gantry_input.parse::<u16>() {
                    let mut s = self.state.write();
                    s.console_params.gantry_angle = angle.min(359);
                    s.editing_taking_place = true;
                }
                // Move to field size
                self.current_field = InputField::FieldSize;
            }
            _ => {}
        }
    }

    fn handle_field_size_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                // Only allow digits and decimal point
                if self.field_y_input.is_empty() && !self.field_x_input.contains('x') {
                    // Still entering X dimension
                    self.field_x_input.push(c);
                } else {
                    // Entering Y dimension
                    self.field_y_input.push(c);
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                // 'x' acts as separator between X and Y dimensions
                if !self.field_x_input.is_empty() && self.field_y_input.is_empty() {
                    // Transition from X to Y dimension
                    // Don't store the 'x', just use it as a signal
                    // The display will show it between the values
                }
            }
            KeyCode::Backspace => {
                if !self.field_y_input.is_empty() {
                    self.field_y_input.pop();
                } else if !self.field_x_input.is_empty() {
                    self.field_x_input.pop();
                }
            }
            KeyCode::Enter => {
                if self.field_x_input.is_empty() {
                    // Copy from reference
                    let s = self.state.read();
                    self.field_x_input = s.reference_params.field_size_x.to_string();
                    self.field_y_input = s.reference_params.field_size_y.to_string();
                }

                // Parse and set field sizes
                // If only X dimension is entered, use it for both X and Y (square field)
                if let Ok(size_x) = self.field_x_input.parse::<f32>() {
                    let size_y = if self.field_y_input.is_empty() {
                        size_x // Square field if Y not specified
                    } else {
                        self.field_y_input.parse::<f32>().unwrap_or(size_x)
                    };

                    let mut s = self.state.write();
                    s.console_params.field_size_x = size_x.max(1.0).min(40.0);
                    s.console_params.field_size_y = size_y.max(1.0).min(40.0);
                    s.editing_taking_place = true;
                    let field_x = s.console_params.field_size_x;
                    let field_y = s.console_params.field_size_y;
                    s.add_log(format!("Field size set to {}×{} cm", field_x, field_y));
                }
                // Move to dose field
                self.current_field = InputField::Dose;
            }
            _ => {}
        }
    }

    fn handle_dose_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                self.dose_input.push(c);
            }
            KeyCode::Backspace => {
                self.dose_input.pop();
            }
            KeyCode::Enter => {
                if self.dose_input.is_empty() {
                    // Copy from reference
                    let s = self.state.read();
                    self.dose_input = s.reference_dose_target.to_string();
                }

                // Parse and set dose
                if let Ok(dose_val) = self.dose_input.parse::<f64>() {
                    let mut s = self.state.write();
                    s.dose_target = dose_val;
                    s.add_log(format!("Dose target set to {} cGy", dose_val));
                }
                // Move to command field
                self.current_field = InputField::Command;
            }
            _ => {}
        }
    }

    fn handle_command_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Enter => {
                self.execute_command();
                self.command_input.clear();
            }
            KeyCode::Esc => {
                // Clear command and go back to mode field
                self.command_input.clear();
                self.current_field = InputField::Mode;
            }
            _ => {}
        }
    }

    fn execute_command(&mut self) {
        let cmd = self.command_input.to_lowercase();

        match cmd.as_str() {
            "t" | "treat" => {
                // Complete data entry and start treatment
                complete_data_entry(self.state.clone());
                start_treatment(self.state.clone());

                // Clear inputs and return to mode field
                self.mode_input.clear();
                self.energy_input.clear();
                self.gantry_input.clear();
                self.field_x_input.clear();
                self.field_y_input.clear();
                self.dose_input.clear();
                self.current_field = InputField::Mode;
            }
            "r" | "reset" => {
                // Reset system and generate new reference
                let mut s = self.state.write();
                s.reset();

                // Clear inputs
                self.mode_input.clear();
                self.energy_input.clear();
                self.gantry_input.clear();
                self.field_x_input.clear();
                self.field_y_input.clear();
                self.dose_input.clear();
                self.current_field = InputField::Mode;
            }
            "p" | "proceed" => {
                // Complete data entry (for setup)
                complete_data_entry(self.state.clone());

                // Clear inputs and return to mode field
                self.mode_input.clear();
                self.energy_input.clear();
                self.gantry_input.clear();
                self.field_x_input.clear();
                self.field_y_input.clear();
                self.dose_input.clear();
                self.current_field = InputField::Mode;
            }
            "s" | "stop" => {
                // Stop treatment
                stop_treatment(self.state.clone());
            }
            "c" | "continue" => {
                // Resume treatment
                resume_treatment(self.state.clone());
            }
            "q" | "quit" => {
                self.should_quit = true;
            }
            "" => {
                // Empty command, just return to mode field
                self.current_field = InputField::Mode;
            }
            _ => {
                let mut s = self.state.write();
                s.add_log(format!("Unknown command: '{}'. Use t/r/p/s/c/q", cmd));
            }
        }
    }

    fn ui(&self, f: &mut Frame) {
        if self.help_visible {
            self.render_help(f);
            return;
        }

        let state = self.state.read();

        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(8),  // Prescription (increased for more params)
                Constraint::Length(16), // Data Entry Form (increased for all fields)
                Constraint::Length(8),  // System Status (increased for more info)
                Constraint::Length(8),  // Hardware State (increased for treatment params)
                Constraint::Min(5),     // Log
                Constraint::Length(2),  // Help hint
            ])
            .split(f.area());

        // Title
        self.render_title(f, chunks[0]);

        // Prescription (reference parameters)
        self.render_prescription(f, chunks[1], &state);

        // Data Entry Form
        self.render_data_entry(f, chunks[2], &state);

        // System Status
        self.render_status(f, chunks[3], &state);

        // Hardware State
        self.render_hardware(f, chunks[4], &state);

        // Log
        self.render_log(f, chunks[5], &state);

        // Help hint
        self.render_help_hint(f, chunks[6]);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("THERAC-25 RADIATION THERAPY SYSTEM")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Double));
        f.render_widget(title, area);
    }

    fn render_prescription(&self, f: &mut Frame, area: Rect, state: &TheracState) {
        let text = vec![
            Line::from(vec![
                Span::styled("PRESCRIPTION: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} @ {}",
                    state.reference_meos.beam_type,
                    state.reference_meos.beam_energy)),
            ]),
            Line::from(vec![
                Span::styled("  Parameters: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("Gantry {}° | Field {}×{} cm | {} cGy @ {:.0} cGy/min",
                    state.reference_params.gantry_angle,
                    state.reference_params.field_size_x,
                    state.reference_params.field_size_y,
                    state.reference_dose_target,
                    state.reference_params.dose_rate)),
            ]),
            Line::from(Span::styled(
                "Press ENTER on numeric fields to copy from prescription",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
            )),
        ];

        let block = Paragraph::new(text)
            .block(Block::default()
                .title("Treatment Plan")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)));
        f.render_widget(block, area);
    }

    fn render_data_entry(&self, f: &mut Frame, area: Rect, _state: &TheracState) {
        let mode_style = if self.current_field == InputField::Mode {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let energy_style = if self.current_field == InputField::Energy {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let gantry_style = if self.current_field == InputField::Gantry {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let field_style = if self.current_field == InputField::FieldSize {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let dose_style = if self.current_field == InputField::Dose {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let command_style = if self.current_field == InputField::Command {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        let field_display = if self.field_x_input.is_empty() && self.field_y_input.is_empty() {
            String::new()
        } else if self.field_y_input.is_empty() {
            format!("{}×", self.field_x_input)
        } else {
            format!("{}×{}", self.field_x_input, self.field_y_input)
        };

        let text = vec![
            Line::from(vec![
                Span::raw("Mode (X=X-ray, E=Electron): "),
                Span::styled(&self.mode_input, mode_style),
                if self.current_field == InputField::Mode {
                    Span::styled("█", mode_style)
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Energy (5/10/15/20/25 MeV): "),
                Span::styled(&self.energy_input, energy_style),
                if self.current_field == InputField::Energy {
                    Span::styled("█", energy_style)
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Gantry Angle (0-360 deg):   "),
                Span::styled(&self.gantry_input, gantry_style),
                if self.current_field == InputField::Gantry {
                    Span::styled("█", gantry_style)
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Field Size (X×Y cm):        "),
                Span::styled(&field_display, field_style),
                if self.current_field == InputField::FieldSize {
                    Span::styled("█", field_style)
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Dose (cGy):                 "),
                Span::styled(&self.dose_input, dose_style),
                if self.current_field == InputField::Dose {
                    Span::styled("█", dose_style)
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Command:                    "),
                Span::styled(&self.command_input, command_style),
                if self.current_field == InputField::Command {
                    Span::styled("█", command_style)
                } else {
                    Span::raw("")
                },
            ]),
        ];

        let block = Paragraph::new(text)
            .block(Block::default()
                .title("Data Entry")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)));
        f.render_widget(block, area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect, state: &TheracState) {
        let phase_color = match state.phase {
            TPhase::Reset => Color::Gray,
            TPhase::DataEntry => Color::Yellow,
            TPhase::SetupTest | TPhase::SetupDone => Color::Blue,
            TPhase::PatientTreatment => Color::Green,
            TPhase::PauseTreatment => Color::Red,
            TPhase::TerminateTreatment => Color::Magenta,
            TPhase::DateTimeIdChanges => Color::Cyan,
        };

        let safety_status = if state.hardware_meos.is_safe() {
            ("SAFE", Color::Green)
        } else {
            ("UNSAFE!", Color::Red)
        };

        let dose_percent = if state.dose_target > 0.0 {
            (state.dose_delivered / state.dose_target * 100.0).min(100.0)
        } else {
            0.0
        };

        let mut text = vec![
            Line::from(vec![
                Span::raw("Phase: "),
                Span::styled(
                    format!("{:?}", state.phase),
                    Style::default().fg(phase_color).add_modifier(Modifier::BOLD)
                ),
                Span::raw("  |  Safety: "),
                Span::styled(
                    safety_status.0,
                    Style::default().fg(safety_status.1).add_modifier(Modifier::BOLD)
                ),
                Span::raw(format!("  |  Malfunctions: {}", state.malfunction_count)),
            ]),
            Line::from(vec![
                Span::styled("Console: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{} @ {}  |  Gantry {}°  |  Field {}×{} cm  |  {} cGy",
                    state.console_meos.beam_type,
                    state.console_meos.beam_energy,
                    state.console_params.gantry_angle,
                    state.console_params.field_size_x,
                    state.console_params.field_size_y,
                    state.dose_target)),
            ]),
        ];

        if let Some(ref malfunction) = state.last_malfunction {
            text.push(Line::from(Span::styled(
                malfunction.as_str(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            )));
        }

        let status = Paragraph::new(text)
            .block(Block::default().title("System Status").borders(Borders::ALL));
        f.render_widget(status, area);

        // Dose gauge
        let gauge_area = Rect {
            x: area.x + 2,
            y: area.y + 4,
            width: area.width - 4,
            height: 2,
        };

        let gauge = Gauge::default()
            .block(Block::default().title("Dose Progress"))
            .gauge_style(
                Style::default()
                    .fg(if dose_percent > 100.0 { Color::Red } else { Color::Green })
                    .bg(Color::Black)
            )
            .percent(dose_percent as u16)
            .label(format!("{:.1}/{:.1} cGy ({:.1}%)",
                state.dose_delivered, state.dose_target, dose_percent));
        f.render_widget(gauge, gauge_area);
    }

    fn render_hardware(&self, f: &mut Frame, area: Rect, state: &TheracState) {
        let hardware_text = vec![
            Line::from(format!("Type: {}  |  Energy: {}  |  Collimator: {}",
                state.hardware_meos.beam_type,
                state.hardware_meos.beam_energy,
                state.hardware_meos.collimator)),
            Line::from(format!("Gantry: {}°  |  Field: {}×{} cm  |  Dose Rate: {:.0} cGy/min",
                state.hardware_params.gantry_angle,
                state.hardware_params.field_size_x,
                state.hardware_params.field_size_y,
                state.hardware_params.dose_rate)),
            Line::from(vec![
                Span::raw("Configuration: "),
                Span::styled(
                    if state.hardware_meos.is_safe() { "SAFE" } else { "UNSAFE!" },
                    Style::default().fg(
                        if state.hardware_meos.is_safe() { Color::Green } else { Color::Red }
                    ).add_modifier(Modifier::BOLD)
                ),
                if state.editing_taking_place {
                    Span::styled("  |  EDITING", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else {
                    Span::raw("")
                },
            ]),
        ];

        let hardware_block = Paragraph::new(hardware_text)
            .block(Block::default()
                .title("Hardware State")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(hardware_block, area);
    }

    fn render_log(&self, f: &mut Frame, area: Rect, state: &TheracState) {
        let log_items: Vec<ListItem> = state
            .log
            .iter()
            .rev()
            .take(area.height as usize - 2)
            .map(|msg| {
                let style = if msg.contains("MALFUNCTION") || msg.contains("CRITICAL") {
                    Style::default().fg(Color::Red)
                } else if msg.contains("complete") || msg.contains("reached") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                ListItem::new(msg.as_str()).style(style)
            })
            .collect();

        let log_list = List::new(log_items)
            .block(Block::default().title("Event Log").borders(Borders::ALL));
        f.render_widget(log_list, area);
    }

    fn render_help_hint(&self, f: &mut Frame, area: Rect) {
        let help_text = Paragraph::new("Commands: (t)reat | (r)eset | (p)roceed | (s)top | (c)ontinue | (q)uit  |  F1=Help")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(help_text, area);
    }

    fn render_help(&self, f: &mut Frame) {
        let help_text = vec![
            Line::from(Span::styled("THERAC-25 SIMULATOR - HELP", Style::default().add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from("DATA ENTRY WORKFLOW:"),
            Line::from("  1. Enter Mode: X (X-ray) or E (Electron)"),
            Line::from("     - X-ray automatically sets energy to 25 MeV and skips to Gantry"),
            Line::from("  2. Enter Energy: 5, 10, 15, 20, or 25 (MeV)"),
            Line::from("  3. Enter Gantry Angle: 0-360 degrees"),
            Line::from("  4. Enter Field Size: Type X value, press 'x', type Y value"),
            Line::from("     - Example: 10x15 for 10cm × 15cm field"),
            Line::from("  5. Enter Dose: target dose in cGy (centigray)"),
            Line::from("  6. Enter Command at prompt"),
            Line::from(""),
            Line::from("COPYING PRESCRIPTION VALUES:"),
            Line::from("  - Press ENTER on any numeric field to copy from prescription"),
            Line::from("  - This simulates the quick-entry workflow that led to real accidents"),
            Line::from("  - Quick entry was convenient but dangerous!"),
            Line::from(""),
            Line::from("FIELD NAVIGATION:"),
            Line::from("  - Press ENTER to advance to next field"),
            Line::from("  - Press ESC to return to Mode entry"),
            Line::from("  - Backspace to delete characters"),
            Line::from(""),
            Line::from("COMMANDS:"),
            Line::from("  t, treat    - Complete entry and start treatment immediately"),
            Line::from("  r, reset    - Reset system and generate new prescription"),
            Line::from("  p, proceed  - Complete data entry and move to setup phase"),
            Line::from("  s, stop     - Pause current treatment"),
            Line::from("  c, continue - Resume paused treatment"),
            Line::from("  q, quit     - Exit simulator"),
            Line::from(""),
            Line::from(Span::styled("THE RACE CONDITION:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("The original Therac-25 bug occurred when operators:"),
            Line::from("  1. Entered X-ray mode (auto-sets to 25 MeV high energy)"),
            Line::from("  2. Noticed mistake, quickly changed to Electron mode"),
            Line::from("  3. Started treatment before hardware collimator sync completed"),
            Line::from("  4. Result: High-energy beam without flatness filter = 100x overdose"),
            Line::from(""),
            Line::from("TO TRIGGER THE BUG:"),
            Line::from("  - Type X (X-ray mode), then immediately press Backspace"),
            Line::from("  - Type E (Electron mode), fill in parameters quickly"),
            Line::from("  - Use 't' command to treat before hardware finishes syncing"),
            Line::from("  - Watch for MALFUNCTION 54 or CRITICAL SAFETY VIOLATION!"),
            Line::from(""),
            Line::from("Press any key to close help..."),
        ];

        let help_block = Paragraph::new(help_text)
            .block(Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_type(BorderType::Double))
            .style(Style::default().bg(Color::Black));

        let area = centered_rect(85, 95, f.area());
        f.render_widget(Block::default().style(Style::default().bg(Color::Black)), f.area());
        f.render_widget(help_block, area);
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
