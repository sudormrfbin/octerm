use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{app::App, github::GitHub};

pub fn draw_ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let area = f.size();
    let notif_area = Rect {
        x: area.x,
        y: area.y,
        height: area.height.saturating_sub(1),
        width: area.width,
    };
    let status_area = Rect {
        x: area.x,
        y: notif_area.bottom(),
        height: 1,
        width: area.width,
    };
    draw_notifications(f, app, notif_area);
    draw_status(f, app, status_area);
}

pub fn draw_status<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let (msg, status) = match &app.state.status_message {
        Some(s) => s,
        None => return,
    };
    let msg_color = match status.as_str() {
        "info" => Color::Blue,
        "error" => Color::Red,
        _ => unreachable!(),
    };
    let paragraph = Paragraph::new(msg.as_str()).style(Style::default().fg(msg_color));
    f.render_widget(paragraph, area);
}

pub fn draw_notifications<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let notifications = app.github.notif.unread().unwrap_or_default();

    let selected_notif_idx = app.state.selected_notification_index;
    let offset = selected_notif_idx // 6 for border, header, padding
        .saturating_sub(area.height.saturating_sub(6).into());

    let notifications: Vec<_> = notifications
        .iter()
        .skip(offset)
        .enumerate()
        .map(|(i, notif)| {
            let (type_, type_color) = match notif.subject.type_.as_str() {
                "Issue" => ("", Color::LightGreen),
                "PullRequest" => ("", Color::LightMagenta),
                "CheckSuite" => ("", Color::Red),
                "Release" => ("", Color::Blue),
                "Discussion" => ("", Color::Yellow),
                _ => ("", Color::White),
            };

            let mut type_style = Style::default().fg(type_color);
            let mut repo_style = Style::default();
            let mut row_style = Style::default();

            if i == selected_notif_idx.saturating_sub(offset) {
                row_style = row_style.bg(Color::Rgb(62, 68, 82));
            };
            if !notif.unread {
                // row_style = row_style.add_modifier(Modifier::DIM);
                type_style = type_style.fg(Color::DarkGray);
                repo_style = repo_style.fg(Color::DarkGray);
            }

            let title = notif.subject.title.as_str();
            Row::new(vec![
                Cell::from(GitHub::repo_name(&notif.repository)).style(repo_style),
                Cell::from(format!("{type_} {title}")).style(type_style),
            ])
            .style(row_style)
        })
        .collect();

    let table_title = format!("Notifications ({})", app.github.notif.len());
    let table = Table::new(notifications)
        .header(
            Row::new(vec!["Repo", "Notification"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().title(table_title).borders(Borders::ALL))
        .widths(&[Constraint::Percentage(20), Constraint::Percentage(80)])
        .style(Style::default().fg(Color::White));

    f.render_widget(table, area);
}
