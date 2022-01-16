use std::time::{Duration, Instant};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use crate::{error::Result, github::GitHub};

pub struct App {
    github: GitHub,
    should_quit: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            github: GitHub::token_from_env()?,
            should_quit: false,
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

            if self.should_quit {
                return Ok(());
            }
        }
    }

    fn on_key(&mut self, key: char) {
        match key {
            'q' => self.should_quit = true,
            _ => (),
        }
    }
}

mod ui {
    use tui::{
        backend::Backend,
        layout::Rect,
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, List, ListItem},
        Frame,
    };

    use crate::app::App;

    pub fn draw_notifications<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
        let notifications = app.github.notifications();
        if let Err(_) = notifications {
            let block = Block::default().title("Error").borders(Borders::ALL);
            f.render_widget(block, area);
            return;
        }
        let notifications: Vec<_> = notifications
            .unwrap()
            .into_iter()
            .map(|n| {
                let repo = &n.repository.name;
                let repo_author = n
                    .repository
                    .owner
                    .as_ref()
                    .map(|o| o.login.clone())
                    .unwrap_or_default();
                let title = &n.subject.title;
                ListItem::new(format!("{repo_author}/{repo}: {title}"))
            })
            .collect();
        let list = List::new(notifications)
            .block(Block::default().title("Notifications").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>");

        f.render_widget(list, area);
    }
}
