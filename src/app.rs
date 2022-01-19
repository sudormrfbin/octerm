use std::time::{Duration, Instant};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use octocrab::Octocrab;
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use crate::{error::Result, github::GitHub, ui};

pub struct App<'a> {
    pub github: GitHub<'a>,
    pub state: AppState,
}

pub struct AppState {
    pub should_quit: bool,
    pub selected_notification_index: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            should_quit: false,
            selected_notification_index: 0,
        }
    }
}

impl<'a> App<'a> {
    pub fn new(octocrab_: &'a Octocrab) -> Result<Self> {
        Ok(Self {
            github: GitHub::new(octocrab_)?,
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
        // TODO: Display error
        let _ = crate::keybind::actions::open_in_browser(self);
    }

    fn on_key(&mut self, key: char) {
        use crate::keybind::actions;
        let _ = match key { // TODO: Display error
            'q' => actions::quit(self),
            'd' => actions::mark_as_read(self),
            'R' => actions::refresh(self),
            'g' => actions::goto_begin(self),
            'G' => actions::goto_end(self),
            'j' => actions::next_item(self),
            'k' => actions::previous_item(self),
            _ => return,
        };
    }
}
