use std::time::{Duration, Instant};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            should_quit: false,
            reload_notifications: false,
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

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

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

    fn on_key(&mut self, key: char) {
        match key {
            'q' => self.state.should_quit = true,
            'r' => self.state.reload_notifications = true,
            _ => (),
        }
    }
}

mod ui {
    use tui::{Frame, backend::Backend, layout::{Constraint, Rect}, style::{Color, Modifier, Style}, widgets::{Block, Borders, Cell, Row, Table}};

    use crate::app::App;

    pub fn draw_notifications<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
        let notifications = app.github.notifications(app.state.reload_notifications);
        if app.state.reload_notifications {
            app.state.reload_notifications = false
        }

        if let Err(_) = notifications {
            let block = Block::default().title("Error").borders(Borders::ALL);
            f.render_widget(block, area);
            return;
        }

        let notifications: Vec<_> = notifications
            .unwrap()
            .into_iter()
            .map(|n| {
                let repo = n.repository.name.as_str();
                let (type_, type_color) = match n.subject.type_.as_str() {
                    "Issue" => ("", Color::LightGreen),
                    "PullRequest" => ("", Color::LightMagenta),
                    "CheckSuite" => ("", Color::Red),
                    "Release" => ("", Color::Blue),
                    "Discussion" => ("", Color::Yellow),
                    _ => ("", Color::Yellow),
                };
                let repo_author = n
                    .repository
                    .owner
                    .as_ref()
                    .map(|o| o.login.clone())
                    .unwrap_or_default();
                let title = n.subject.title.as_str();
                Row::new(vec![
                    Cell::from(format!("{repo_author}/{repo}")),
                    Cell::from(format!("{type_} {title}")).style(Style::default().fg(type_color)),
                ])
            })
            .collect();
        let title = format!("Notifications ({})", notifications.len());
        let table = Table::new(notifications)
            .header(Row::new(vec!["Repo", "Notification"]).style(Style::default().add_modifier(Modifier::BOLD)))
            .block(Block::default().title(title).borders(Borders::ALL))
            .widths(&[Constraint::Percentage(20), Constraint::Percentage(80)])
            .style(Style::default().fg(Color::White));

        f.render_widget(table, area);
    }
}
