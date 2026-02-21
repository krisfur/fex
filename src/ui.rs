use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, SearchState};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar (at top, with border)
            Constraint::Length(1), // status bar
            Constraint::Min(0),    // results (fills remaining space)
        ])
        .split(area);

    render_search(f, app, chunks[0]);
    render_status(f, app, chunks[1]);
    render_results(f, app, chunks[2]);
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    // Work in chars to avoid splitting UTF-8 sequences
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let cut = max.saturating_sub(3);
        let truncated: String = chars[..cut].iter().collect();
        format!("{truncated}...")
    }
}

fn render_results(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let visible_count = (area.height as usize) / 2;
    let width = area.width as usize;

    let mut lines: Vec<Line> = Vec::new();

    for i in 0..visible_count {
        let pkg_idx = app.scroll_offset + i;
        if pkg_idx >= app.packages.len() {
            // Empty filler lines so layout is stable
            lines.push(Line::raw(""));
            lines.push(Line::raw(""));
            continue;
        }
        let pkg = &app.packages[pkg_idx];
        let is_selected = pkg_idx == app.selected;

        let source_color = app.provider.source_color(&pkg.source);

        if is_selected {
            // Source badge keeps its color; the rest gets REVERSED
            let source_span =
                Span::styled(format!("[{}]", pkg.source), Style::new().fg(source_color));
            let installed_span = if pkg.installed {
                Span::styled(
                    " *",
                    Style::new().fg(Color::Green).add_modifier(Modifier::REVERSED),
                )
            } else {
                Span::styled("  ", Style::new().add_modifier(Modifier::REVERSED))
            };
            let name_span = Span::styled(
                format!(" {}", pkg.name),
                Style::new()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::REVERSED),
            );
            let version_part = if pkg.version.is_empty() {
                String::new()
            } else {
                format!(" {}", pkg.version)
            };
            let version_span =
                Span::styled(version_part, Style::new().add_modifier(Modifier::REVERSED));
            // Trailing padding to fill the line with reversed background
            let header_text = format!(
                "[{}]{}{}{}",
                pkg.source,
                if pkg.installed { " *" } else { "  " },
                pkg.name,
                if pkg.version.is_empty() { String::new() } else { format!(" {}", pkg.version) }
            );
            let pad_len = width.saturating_sub(header_text.len());
            let pad_span = Span::styled(
                " ".repeat(pad_len),
                Style::new().add_modifier(Modifier::REVERSED),
            );
            lines.push(Line::from(vec![
                source_span,
                installed_span,
                name_span,
                version_span,
                pad_span,
            ]));

            // Description line (selected)
            let indent = "         "; // 9 spaces to match C++ DESC indent
            let max_desc = width.saturating_sub(9);
            let desc = truncate(&pkg.description, max_desc);
            let desc_pad = " ".repeat(width.saturating_sub(9 + desc.len()));
            lines.push(Line::from(vec![
                Span::styled(format!("{indent}{desc}"), Style::new().add_modifier(Modifier::REVERSED)),
                Span::styled(desc_pad, Style::new().add_modifier(Modifier::REVERSED)),
            ]));
        } else {
            // Normal (unselected) line
            let source_span =
                Span::styled(format!("[{}]", pkg.source), Style::new().fg(source_color));
            let installed_span = if pkg.installed {
                Span::styled(" *", Style::new().fg(Color::Green))
            } else {
                Span::raw("  ")
            };
            let name_span =
                Span::styled(format!(" {}", pkg.name), Style::new().add_modifier(Modifier::BOLD));
            let version_part = if pkg.version.is_empty() {
                String::new()
            } else {
                format!(" {}", pkg.version)
            };
            let version_span = Span::raw(version_part);
            lines.push(Line::from(vec![
                source_span,
                installed_span,
                name_span,
                version_span,
            ]));

            // Description line
            let indent = "         ";
            let max_desc = width.saturating_sub(9);
            let desc = truncate(&pkg.description, max_desc);
            lines.push(Line::raw(format!("{indent}{desc}")));
        }
    }

    let results_widget = Paragraph::new(Text::from(lines));
    f.render_widget(results_widget, area);
}

fn render_status(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let provider_name = app.provider.name();
    let n = app.packages.len();

    let search_indicator = match app.search_state {
        SearchState::Searching => " Searching...",
        SearchState::Done => " Ready",
        SearchState::Idle => "",
    };

    let status_text = format!(
        " provider: {provider_name} │ {n} results{search_indicator}"
    );

    let style = match app.search_state {
        SearchState::Searching => Style::new().fg(Color::Yellow),
        SearchState::Done if !app.packages.is_empty() => Style::new().fg(Color::Green),
        _ => Style::new(),
    };

    // Show error in red if the status message starts with "Error:"
    let final_style = if app.status_message.starts_with("Error:") {
        Style::new().fg(Color::Red)
    } else {
        style
    };

    let msg = if app.status_message.starts_with("Error:")
        || app.status_message == "No results found."
        || app.status_message == "Start typing to search."
    {
        format!(" {}", app.status_message)
    } else {
        status_text
    };

    let para = Paragraph::new(msg).style(final_style);
    f.render_widget(para, area);
}

fn render_search(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Search ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Show query with a block cursor character
    let display = format!("{}█", app.query);
    let para = Paragraph::new(display).style(Style::new().add_modifier(Modifier::BOLD));
    f.render_widget(para, inner);
}
