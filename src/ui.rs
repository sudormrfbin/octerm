use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Spans, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::{
    app::{App, StatusLine},
    github::{GitHub, IssueState, Notification, NotificationTarget, PullRequestState},
};

#[derive(PartialEq)]
pub enum Route {
    Notifications,
    NotifTarget(Notification),
}

pub fn draw_ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let area = f.size();
    let route_area = Rect {
        x: area.x,
        y: area.y,
        height: area.height.saturating_sub(1),
        width: area.width,
    };
    let status_area = Rect {
        x: area.x,
        y: route_area.bottom(),
        height: 1,
        width: area.width,
    };

    match app.state.route {
        Route::Notifications => draw_notifications(f, app, route_area),
        Route::NotifTarget(_) => draw_notif_target(f, app, route_area),
    }
    draw_statusline(f, app, status_area);
}

macro_rules! span {
    ($text:expr) => {
        tui::text::Span::from($text)
    };
    ($text:expr, $(fg:$fg:ident)?, $(bg:$bg:ident)?) => {
        {
            let mut style = tui::style::Style::default();
            $( style = style.fg(tui::style::Color::$fg); )?
            $( style = style.bg(tui::style::Color::$bg); )?
            tui::text::Span::styled($text, style)
        }
    };
    ($text:expr, $style:expr) => {
        {
            tui::text::Span::styled($text, $style)
        }
    };
}

fn draw_notif_target<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let notif = match app.state.route {
        Route::NotifTarget(ref notif) => notif,
        Route::Notifications => unreachable!(),
    };

    let target_color = notif_target_color(&notif.target);
    let icon_style = Style::default().fg(target_color);
    let target_style = Style::default().bg(target_color).fg(Color::Black);
    let title = match notif.target {
        NotificationTarget::Issue(ref issue) => Spans::from(
            [
                span!(issue.icon(), icon_style),
                span!(" "),
                span!(issue.title.as_str()),
                span!(format!(" #{} ", issue.unique), fg: DarkGray,),
                span!(issue.state.to_string(), target_style),
            ]
            .to_vec(),
        ),
        NotificationTarget::PullRequest(ref pr) => Spans::from(
            [
                span!(pr.icon(), icon_style),
                span!(" "),
                span!(pr.title.as_str()),
                span!(format!(" #{} ", pr.unique), fg: DarkGray,),
                span!(pr.state.to_string(), target_style),
            ]
            .to_vec(),
        ),
        NotificationTarget::Release(ref release) => Spans::from(
            [
                span!(release.title.as_str()),
                span!(release.unique.as_str(), fg: DarkGray,),
            ]
            .to_vec(),
        ),

        NotificationTarget::Discussion => "This is a discussion".into(),
        NotificationTarget::CiBuild => "This is a CI Build".into(),
        NotificationTarget::Unknown => "This is an unknown item".into(),
    };

    let body: tui::text::Text = match notif.target {
        NotificationTarget::Issue(ref issue) => issue.body.as_str(),
        NotificationTarget::PullRequest(ref pr) => pr.body.as_str(),
        NotificationTarget::Release(ref release) => release.body.as_str(),
        _ => "No description provided.",
    }
    .into();

    let mut content = Text::from(title);
    content.extend(Text::raw("\n"));
    content.extend(body);

    let para = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .wrap(tui::widgets::Wrap { trim: false });
    f.render_widget(para, area);
}

pub fn draw_statusline<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let (msg, severity) = match &app.state.statusline {
        StatusLine::Empty => return,
        StatusLine::Loading => (
            format!("{} Loading", app.state.spinner.next()),
            "info".to_string(),
        ),
        StatusLine::Text { content, severity } => (content.clone(), severity.clone()),
    };
    let msg_color = match severity.as_str() {
        "info" => Color::Blue,
        "error" => Color::Red,
        _ => unreachable!("'{severity}' is an invalid severity for statusline"),
    };
    let paragraph = Paragraph::new(msg).style(Style::default().fg(msg_color));
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
            let icon = notif.target.icon();
            let type_color = notif_target_color(&notif.target);

            let mut type_style = Style::default().fg(type_color);
            let mut repo_style = Style::default();
            let mut row_style = Style::default();

            if i == selected_notif_idx.saturating_sub(offset) {
                row_style = row_style.bg(Color::Rgb(62, 68, 82));
            };
            if !notif.inner.unread {
                // row_style = row_style.add_modifier(Modifier::DIM);
                type_style = type_style.fg(Color::DarkGray);
                repo_style = repo_style.fg(Color::DarkGray);
            }

            let title = notif.inner.subject.title.as_str();
            Row::new(vec![
                Cell::from(GitHub::repo_name(&notif.inner.repository)).style(repo_style),
                Cell::from(format!("{icon} {title}")).style(type_style),
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

fn notif_target_color(target: &NotificationTarget) -> Color {
    match target {
        NotificationTarget::Issue(ref issue) => match issue.state {
            IssueState::Open => Color::LightGreen,
            IssueState::Closed => Color::Red,
        },
        NotificationTarget::PullRequest(ref pr) => match pr.state {
            PullRequestState::Open => Color::Green,
            PullRequestState::Merged => Color::Magenta,
            PullRequestState::Closed => Color::Red,
        },
        NotificationTarget::CiBuild => Color::Red,
        NotificationTarget::Release(_) => Color::Blue,
        NotificationTarget::Discussion => Color::Yellow,
        NotificationTarget::Unknown => Color::White,
    }
}
