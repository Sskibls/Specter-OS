// PhantomKernel TUI - UI Rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use crate::app::{App, ShardStatus, Tab};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tabs
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Status bar
        ])
        .split(area);

    render_tabs(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::titles().iter().map(|t| Line::from(*t)).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" PhantomKernel OS "))
        .select(Tab::titles().iter().position(|&t| t == app.tab_name()).unwrap_or(0))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.current_tab {
        Tab::Dashboard => render_dashboard(frame, app, area),
        Tab::Shards => render_shards(frame, app, area),
        Tab::Network => render_network(frame, app, area),
        Tab::Settings => render_settings(frame, app, area),
    }
}

fn render_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: System status
    let status_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Panic Mode: {}", if app.panic_mode { "⚠️ ACTIVE" } else { "Off" }),
            if app.panic_mode { Style::default().fg(Color::Red).add_modifier(Modifier::BOLD) } else { Style::default() },
        )),
        Line::from(Span::styled(
            format!("Mask Mode: {}", if app.mask_mode { "🎭 ACTIVE" } else { "Off" }),
            if app.mask_mode { Style::default().fg(Color::Yellow) } else { Style::default() },
        )),
        Line::from(Span::styled(
            format!("Travel Mode: {}", if app.travel_mode { "✈️ ACTIVE" } else { "Off" }),
            if app.travel_mode { Style::default().fg(Color::Green) } else { Style::default() },
        )),
        Line::from(Span::styled(
            format!("Kill Switch: {}", if app.kill_switch { "🚫 ENGAGED" } else { "Off" }),
            if app.kill_switch { Style::default().fg(Color::Red).add_modifier(Modifier::BOLD) } else { Style::default() },
        )),
    ];
    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title(" Emergency Modes "));
    frame.render_widget(status, chunks[0]);

    // Right: Recent audit events
    let items: Vec<ListItem> = app.audit_events.iter().take(10).map(|e| ListItem::new(e.as_str())).collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Audit Log "));
    frame.render_widget(list, chunks[1]);
}

fn render_shards(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.shards.iter().map(|shard| {
        let status_icon = match shard.status {
            ShardStatus::Running => "🟢",
            ShardStatus::Stopped => "⚪",
            ShardStatus::Locked => "🔒",
        };
        let text = format!("{} {} - Network: {}", status_icon, shard.name, shard.network);
        let style = match shard.status {
            ShardStatus::Running => Style::default().fg(Color::Green),
            ShardStatus::Stopped => Style::default().fg(Color::White),
            ShardStatus::Locked => Style::default().fg(Color::Red),
        };
        ListItem::new(Line::from(Span::styled(text, style)))
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Persona Shards "));
    frame.render_widget(list, area);
}

fn render_network(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    // Status
    let status_color = if app.network_status.leak_detected {
        Color::Red
    } else if app.kill_switch {
        Color::Red
    } else {
        Color::Green
    };
    let status = Paragraph::new(Span::styled(
        format!("Status: {} | Route: {}", app.network_status.status, app.network_status.route),
        Style::default().fg(status_color).add_modifier(Modifier::BOLD),
    ))
    .block(Block::default().borders(Borders::ALL).title(" Network Status "));
    frame.render_widget(status, chunks[0]);

    // Traffic gauge
    let upload_pct = (app.network_status.upload / 100.0).min(1.0);
    let download_pct = (app.network_status.download / 100.0).min(1.0);
    
    let upload_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Cyan))
        .label(format!("↑ Upload: {:.1} KB/s", app.network_status.upload))
        .ratio(upload_pct);
    frame.render_widget(upload_gauge, chunks[1]);

    let download_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Magenta))
        .label(format!("↓ Download: {:.1} KB/s", app.network_status.download))
        .ratio(download_pct);
    frame.render_widget(download_gauge, chunks[2]);
}

fn render_settings(frame: &mut Frame, _app: &App, area: Rect) {
    let settings = vec![
        Line::from(""),
        Line::from("Keyboard Shortcuts:"),
        Line::from("  [q] Quit"),
        Line::from("  [p] Panic Mode"),
        Line::from("  [m] Mask Mode"),
        Line::from("  [t] Travel Mode"),
        Line::from("  [k] Kill Switch"),
        Line::from("  [Tab] Next Tab"),
        Line::from("  [1-4] Switch Tab"),
        Line::from("  [r] Refresh"),
        Line::from(""),
        Line::from("Theme: Default (TUI)"),
    ];
    let settings = Paragraph::new(settings)
        .block(Block::default().borders(Borders::ALL).title(" Settings "))
        .wrap(Wrap { trim: true });
    frame.render_widget(settings, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        " Shards: {}/4 | Network: {} | Tab: {} | [q]uit [p]anic [h]elp ",
        app.shards.iter().filter(|s| matches!(s.status, ShardStatus::Running)).count(),
        app.network_status.status,
        app.tab_name()
    );
    let status_bar = Paragraph::new(status)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}
