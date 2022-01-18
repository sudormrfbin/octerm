use std::{
    ops::Add,
    time::{Duration, Instant},
};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use crate::{error::Result, github::GitHub};

pub struct App {
    github: GitHub,
    state: AppState,
}

pub struct AppState {
    pub should_quit: bool,
    pub reload_notifications: bool,
    pub selected_notification_index: usize,
    pub notifications_len: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            should_quit: false,
            reload_notifications: false,
            selected_notification_index: 0,
            notifications_len: 0,
        }
    }
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            github: GitHub::token_from_env()?,
            state: AppState::default(),
        })
    }

    pub fn run(self, tick_rate: Duration) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.event_loop(&mut terminal, tick_rate)?;

        // TODO: Add custom panic handler to restore terminal state
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;

        Ok(())
    }

    fn event_loop<B: Backend>(
        mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> Result<()> {
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|f| ui::draw_notifications(f, &mut self, f.size()))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = crossterm::event::read()? {
                    match key.code {
                        KeyCode::Char(c) => self.on_key(c),
                        KeyCode::Enter => self.on_enter(),
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }

            if self.state.should_quit {
                return Ok(());
            }
        }
    }

    fn on_enter(&mut self) {
        let notifs = match self.github.notifications(self.state.reload_notifications) {
            Ok(n) => n,
            Err(_) => return, // TODO: Display error
        };
        let notif = notifs
            .into_iter()
            .nth(self.state.selected_notification_index)
            .unwrap()
            .clone();
        let url = match self.github.open_notification(&notif) {
            Ok(u) => u,
            Err(_) => return, // TODO: Display error
        };
        let _ = open::that(url.as_str()); // TODO: Display error
    }

    fn on_key(&mut self, key: char) {
        let s = &mut self.state;
        match key {
            'q' => s.should_quit = true,
            'r' => s.reload_notifications = true,
            'g' => s.selected_notification_index = 0,
            'G' => s.selected_notification_index = s.notifications_len.saturating_sub(1),
            'j' => {
                s.selected_notification_index = s
                    .selected_notification_index
                    .add(1)
                    .min(s.notifications_len.saturating_sub(1))
            }
            'k' => s.selected_notification_index = s.selected_notification_index.saturating_sub(1),
            _ => (),
        }
    }
}

mod ui {
    use tui::{
        backend::Backend,
        layout::{Constraint, Rect},
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Cell, Paragraph, Row, Table},
        Frame,
    };

    use crate::{app::App, github::GitHub};

    pub fn draw_notifications<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
        let notifications = app.github.notifications(app.state.reload_notifications);
        if app.state.reload_notifications {
            app.state.selected_notification_index = 0;
            app.state.reload_notifications = false;
        }

        let notifications = match notifications {
            Ok(n) => n,
            Err(err) => {
                app.state.notifications_len = 0;
                let paragraph = Paragraph::new(format!("{:?}", err))
                    .block(Block::default().title("Error").borders(Borders::ALL));
                f.render_widget(paragraph, area);
                return;
            }
        };
        app.state.notifications_len = notifications.into_iter().len();

        let selected_notif_idx = app.state.selected_notification_index;
        let offset = selected_notif_idx // 6 for border, header, padding
            .saturating_sub(area.height.saturating_sub(6).into());

        let notifications: Vec<_> = notifications
            .into_iter()
            .skip(offset)
            .enumerate()
            .map(|(i, n)| {
                let (type_, type_color) = match n.subject.type_.as_str() {
                    "Issue" => ("", Color::LightGreen),
                    "PullRequest" => ("", Color::LightMagenta),
                    "CheckSuite" => ("", Color::Red),
                    "Release" => ("", Color::Blue),
                    "Discussion" => ("", Color::Yellow),
                    _ => ("", Color::White),
                };
                let title = n.subject.title.as_str();

                let row_style = if i == selected_notif_idx.saturating_sub(offset) {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(GitHub::repo_name(&n.repository)),
                    Cell::from(format!("{type_} {title}")).style(Style::default().fg(type_color)),
                ])
                .style(row_style)
            })
            .collect();

        let table_title = format!("Notifications ({})", app.state.notifications_len);
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
}
