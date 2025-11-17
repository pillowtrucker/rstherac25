//! Optional TUI rendering module for the Therac-25 simulator
//!
//! This module provides rendering functions that can be used by external
//! applications to display the Therac-25 interface.
//!
//! Only compiled when the "tui-render" feature is enabled.

#[cfg(feature = "tui-render")]
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Paragraph, List, ListItem, Gauge},
};

use crate::state::{SharedTheracState, TPhase, BeamType};

/// Render the Therac-25 interface to a ratatui Frame
/// This can be called from an external TUI application
#[cfg(feature = "tui-render")]
pub fn render_therac25(frame: &mut Frame, state: &SharedTheracState) {
    let state_guard = state.read();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Min(0),      // Main area
            Constraint::Length(3),   // Footer
        ])
        .split(frame.area());

    render_header(frame, chunks[0]);
    render_main_area(frame, chunks[1], &state_guard);
    render_footer(frame, chunks[2], &state_guard);
}

#[cfg(feature = "tui-render")]
fn render_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new("⚡ THERAC-25 RADIATION THERAPY SIMULATOR ⚡")
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Double));
    frame.render_widget(header, area);
}

#[cfg(feature = "tui-render")]
fn render_main_area(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    render_left_panel(frame, chunks[0], state);
    render_right_panel(frame, chunks[1], state);
}

#[cfg(feature = "tui-render")]
fn render_left_panel(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),   // Reference prescription
            Constraint::Length(8),   // Console settings
            Constraint::Length(8),   // Hardware status
            Constraint::Min(0),      // Treatment log
        ])
        .split(area);

    render_reference_prescription(frame, chunks[0], state);
    render_console_settings(frame, chunks[1], state);
    render_hardware_status(frame, chunks[2], state);
    render_log(frame, chunks[3], state);
}

#[cfg(feature = "tui-render")]
fn render_right_panel(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),   // Treatment phase
            Constraint::Length(8),   // Dose delivery
            Constraint::Min(0),      // Status
        ])
        .split(area);

    render_treatment_phase(frame, chunks[0], state);
    render_dose_delivery(frame, chunks[1], state);
    render_status(frame, chunks[2], state);
}

#[cfg(feature = "tui-render")]
fn render_reference_prescription(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let content = vec![
        Line::from(vec![
            Span::raw("Mode: "),
            Span::styled(
                format!("{}", state.reference_meos.beam_type),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Energy: "),
            Span::styled(
                format!("{}", state.reference_meos.beam_energy),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Gantry: "),
            Span::styled(
                format!("{} deg", state.reference_params.gantry_angle),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Field: "),
            Span::styled(
                format!("{}x{} cm", state.reference_params.field_size_x, state.reference_params.field_size_y),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Dose: "),
            Span::styled(
                format!("{} cGy", state.reference_dose_target),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let block = Paragraph::new(content)
        .block(Block::default()
            .title("Reference Prescription")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)));
    frame.render_widget(block, area);
}

#[cfg(feature = "tui-render")]
fn render_console_settings(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let content = vec![
        Line::from(vec![
            Span::raw("Mode: "),
            Span::styled(
                format!("{}", state.console_meos.beam_type),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Energy: "),
            Span::styled(
                format!("{}", state.console_meos.beam_energy),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Gantry: "),
            Span::styled(
                format!("{} deg", state.console_params.gantry_angle),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Field: "),
            Span::styled(
                format!("{}x{} cm", state.console_params.field_size_x, state.console_params.field_size_y),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Dose: "),
            Span::styled(
                format!("{} cGy", state.dose_target),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ];

    let block = Paragraph::new(content)
        .block(Block::default()
            .title("Console Settings")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)));
    frame.render_widget(block, area);
}

#[cfg(feature = "tui-render")]
fn render_hardware_status(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let collimator_color = if state.hardware_meos.is_safe() {
        Color::Green
    } else {
        Color::Red
    };

    let content = vec![
        Line::from(vec![
            Span::raw("Mode: "),
            Span::styled(
                format!("{}", state.hardware_meos.beam_type),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("Energy: "),
            Span::styled(
                format!("{}", state.hardware_meos.beam_energy),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("Collimator: "),
            Span::styled(
                format!("{}", state.hardware_meos.collimator),
                Style::default().fg(collimator_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Safe: "),
            Span::styled(
                if state.hardware_meos.is_safe() { "YES" } else { "NO" },
                Style::default().fg(collimator_color).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let block = Paragraph::new(content)
        .block(Block::default()
            .title("Hardware Status")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)));
    frame.render_widget(block, area);
}

#[cfg(feature = "tui-render")]
fn render_treatment_phase(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let phase_color = match state.phase {
        TPhase::PatientTreatment => Color::Green,
        TPhase::PauseTreatment | TPhase::TerminateTreatment => Color::Red,
        _ => Color::Yellow,
    };

    let content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!("{}", state.phase),
                Style::default().fg(phase_color).add_modifier(Modifier::BOLD),
            ),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(Block::default()
        .title("Treatment Phase")
        .borders(Borders::ALL));

    frame.render_widget(content, area);
}

#[cfg(feature = "tui-render")]
fn render_dose_delivery(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let progress = if state.dose_target > 0.0 {
        (state.dose_delivered / state.dose_target).min(1.0)
    } else {
        0.0
    };

    let gauge = Gauge::default()
        .block(Block::default().title("Dose Progress").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent((progress * 100.0) as u16)
        .label(format!("{:.1}/{:.1} cGy", state.dose_delivered, state.dose_target));

    frame.render_widget(gauge, area);
}

#[cfg(feature = "tui-render")]
fn render_status(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let mut lines = vec![];

    if let Some(ref malfunction) = state.last_malfunction {
        lines.push(Line::from(vec![
            Span::styled(
                "⚠ MALFUNCTION: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                malfunction,
                Style::default().fg(Color::Red),
            ),
        ]));
    }

    if !state.treatment_outcome.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                "Outcome: ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(&state.treatment_outcome),
        ]));
    }

    lines.push(Line::from(format!("Malfunctions: {}", state.malfunction_count)));

    let block = Paragraph::new(lines)
        .block(Block::default()
            .title("Status")
            .borders(Borders::ALL));

    frame.render_widget(block, area);
}

#[cfg(feature = "tui-render")]
fn render_log(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let log_items: Vec<ListItem> = state.log.iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .rev()
        .map(|entry| ListItem::new(entry.clone()))
        .collect();

    let list = List::new(log_items)
        .block(Block::default()
            .title("Treatment Log")
            .borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(list, area);
}

#[cfg(feature = "tui-render")]
fn render_footer(frame: &mut Frame, area: Rect, state: &crate::state::TheracState) {
    let help_text = "F1: Help | Ctrl+C: Quit | Commands: TREAT, RESET, SETUP";
    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, area);
}
