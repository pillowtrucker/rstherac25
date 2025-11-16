//! Terminal User Interface for Therac-25 simulator
//!
//! Provides an interactive TUI for operating the simulated Therac-25 machine

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

pub struct TuiApp {
    state: SharedTheracState,
    should_quit: bool,
    help_visible: bool,
}

impl TuiApp {
    pub fn new(state: SharedTheracState) -> Self {
        Self {
            state,
            should_quit: false,
            help_visible: false,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

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

        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => self.should_quit = true,
            KeyCode::Char('?') | KeyCode::F(1) => self.help_visible = true,
            KeyCode::Char('r') => {
                let mut s = self.state.write();
                s.reset();
            },
            KeyCode::Char('d') => {
                complete_data_entry(self.state.clone());
            },
            KeyCode::Char('s') => {
                start_treatment(self.state.clone());
            },
            KeyCode::Char('p') => {
                stop_treatment(self.state.clone());
            },
            KeyCode::Char('c') => {
                resume_treatment(self.state.clone());
            },
            KeyCode::Char('1') => {
                self.update_beam_type(BeamType::XRay);
            },
            KeyCode::Char('2') => {
                self.update_beam_type(BeamType::Electron);
            },
            KeyCode::Char('5') => {
                self.update_beam_energy(BeamEnergy::E5);
            },
            KeyCode::Char('6') => {
                self.update_beam_energy(BeamEnergy::E10);
            },
            KeyCode::Char('7') => {
                self.update_beam_energy(BeamEnergy::E15);
            },
            KeyCode::Char('8') => {
                self.update_beam_energy(BeamEnergy::E20);
            },
            KeyCode::Char('9') => {
                self.update_beam_energy(BeamEnergy::E25);
            },
            KeyCode::Char('g') => {
                // Generate random safe parameters
                let params = generate_random_parameters();
                update_console_meos(self.state.clone(), params);
            },
            KeyCode::Char('b') => {
                // Generate race condition parameters (bug trigger)
                let current = self.state.read().console_meos;
                let params = generate_race_condition_parameters(current);
                update_console_meos(self.state.clone(), params);
            },
            KeyCode::Char('t') => {
                // Toggle collimator (manual override - dangerous!)
                let mut s = self.state.write();
                s.console_meos.collimator = match s.console_meos.collimator {
                    CollimatorPosition::InPosition => CollimatorPosition::OutOfPosition,
                    CollimatorPosition::OutOfPosition => CollimatorPosition::InPosition,
                    CollimatorPosition::Transitioning => CollimatorPosition::InPosition,
                };
                let collimator_pos = s.console_meos.collimator;
                s.add_log(format!("Collimator manually set to {}", collimator_pos));
            },
            _ => {}
        }
    }

    fn update_beam_type(&mut self, beam_type: BeamType) {
        let mut s = self.state.write();
        if s.phase == TPhase::DataEntry {
            s.console_meos.beam_type = beam_type;
            s.add_log(format!("Beam type set to {}", beam_type));
        }
    }

    fn update_beam_energy(&mut self, energy: BeamEnergy) {
        let mut s = self.state.write();
        if s.phase == TPhase::DataEntry {
            s.console_meos.beam_energy = energy;
            s.add_log(format!("Beam energy set to {}", energy));
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
                Constraint::Length(8),  // Status
                Constraint::Length(10), // Console/Hardware
                Constraint::Min(10),    // Log
                Constraint::Length(3),  // Help hint
            ])
            .split(f.area());

        // Title
        self.render_title(f, chunks[0]);

        // Status
        self.render_status(f, chunks[1], &state);

        // Console and Hardware MEOS
        self.render_meos(f, chunks[2], &state);

        // Log
        self.render_log(f, chunks[3], &state);

        // Help hint
        self.render_help_hint(f, chunks[4]);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("THERAC-25 RADIATION THERAPY SIMULATOR")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Double));
        f.render_widget(title, area);
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

        let dose_percent = if state.dose_target > 0.0 {
            (state.dose_delivered / state.dose_target * 100.0).min(100.0)
        } else {
            0.0
        };

        let safety_status = if state.hardware_meos.is_safe() {
            ("SAFE", Color::Green)
        } else {
            ("UNSAFE!", Color::Red)
        };

        let text = vec![
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
            ]),
            Line::from(vec![
                Span::raw(format!("Malfunctions: {}  |  Class3: {}",
                    state.malfunction_count, state.class3)),
            ]),
        ];

        let status = Paragraph::new(text)
            .block(Block::default().title("System Status").borders(Borders::ALL));
        f.render_widget(status, area);

        // Dose gauge
        let gauge_area = Rect {
            x: area.x + 2,
            y: area.y + 3,
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

        // Last malfunction
        if let Some(ref malfunction) = state.last_malfunction {
            let malfunction_area = Rect {
                x: area.x + 2,
                y: area.y + 5,
                width: area.width - 4,
                height: 1,
            };
            let mal_text = Paragraph::new(malfunction.as_str())
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
            f.render_widget(mal_text, malfunction_area);
        }
    }

    fn render_meos(&self, f: &mut Frame, area: Rect, state: &TheracState) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Console MEOS
        let console_text = vec![
            Line::from(format!("Beam Type: {}", state.console_meos.beam_type)),
            Line::from(format!("Energy: {}", state.console_meos.beam_energy)),
            Line::from(format!("Collimator: {}", state.console_meos.collimator)),
            Line::from(""),
            Line::from(vec![
                Span::raw("Safe: "),
                Span::styled(
                    if state.console_meos.is_safe() { "YES" } else { "NO" },
                    Style::default().fg(
                        if state.console_meos.is_safe() { Color::Green } else { Color::Red }
                    ).add_modifier(Modifier::BOLD)
                ),
            ]),
        ];

        let console_block = Paragraph::new(console_text)
            .block(Block::default()
                .title("Console Parameters (Operator Input)")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)));
        f.render_widget(console_block, chunks[0]);

        // Hardware MEOS
        let hardware_text = vec![
            Line::from(format!("Beam Type: {}", state.hardware_meos.beam_type)),
            Line::from(format!("Energy: {}", state.hardware_meos.beam_energy)),
            Line::from(format!("Collimator: {}", state.hardware_meos.collimator)),
            Line::from(""),
            Line::from(vec![
                Span::raw("Safe: "),
                Span::styled(
                    if state.hardware_meos.is_safe() { "YES" } else { "NO" },
                    Style::default().fg(
                        if state.hardware_meos.is_safe() { Color::Green } else { Color::Red }
                    ).add_modifier(Modifier::BOLD)
                ),
            ]),
        ];

        let hardware_block = Paragraph::new(hardware_text)
            .block(Block::default()
                .title("Hardware State (Actual)")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(hardware_block, chunks[1]);
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
        let help_text = Paragraph::new("Press ? or F1 for help  |  Q or ESC to quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help_text, area);
    }

    fn render_help(&self, f: &mut Frame) {
        let help_text = vec![
            Line::from(Span::styled("THERAC-25 SIMULATOR - HELP", Style::default().add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from("BASIC CONTROLS:"),
            Line::from("  q, ESC       - Quit simulator"),
            Line::from("  ?, F1        - Show this help"),
            Line::from("  r            - Reset system"),
            Line::from(""),
            Line::from("DATA ENTRY MODE:"),
            Line::from("  1            - Set beam type to X-Ray"),
            Line::from("  2            - Set beam type to Electron"),
            Line::from("  5            - Set energy to 5 MeV"),
            Line::from("  6            - Set energy to 10 MeV"),
            Line::from("  7            - Set energy to 15 MeV"),
            Line::from("  8            - Set energy to 20 MeV"),
            Line::from("  9            - Set energy to 25 MeV"),
            Line::from("  t            - Toggle collimator position (manual override)"),
            Line::from("  d            - Complete data entry"),
            Line::from(""),
            Line::from("TREATMENT CONTROLS:"),
            Line::from("  s            - Start treatment"),
            Line::from("  p            - Pause treatment"),
            Line::from("  c            - Continue/resume treatment"),
            Line::from(""),
            Line::from("RANDOM GENERATION:"),
            Line::from("  g            - Generate random safe parameters"),
            Line::from(Span::styled("  b            - Generate bug-triggering parameters (race condition!)",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled("EDUCATIONAL NOTE:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("This simulator intentionally contains the race conditions that caused"),
            Line::from("real-world patient deaths. Use 'b' to quickly change parameters and"),
            Line::from("trigger the bug where hardware state doesn't match console settings."),
            Line::from(""),
            Line::from("Press any key to close help..."),
        ];

        let help_block = Paragraph::new(help_text)
            .block(Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_type(BorderType::Double))
            .style(Style::default().bg(Color::Black));

        let area = centered_rect(80, 90, f.area());
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
