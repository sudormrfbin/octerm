use std::sync::mpsc::Sender;

use crate::{events::NotifEvent, github::GitHub, ui::Route};

pub struct App {
    pub github: GitHub,
    pub state: AppState,
    pub event_tx: Sender<NotifEvent>,
}

pub struct AppState {
    pub open_url: Option<String>,
    pub route: Route,
    pub should_quit: bool,
    pub is_loading: bool,
    pub statusline: StatusLine,
    pub spinner: Spinner,
    pub selected_notification_index: usize,
}

pub struct Spinner {
    frames: Vec<&'static str>,
    idx: usize,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            frames: vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
            idx: 0
        }
    }

    pub fn next(&mut self) -> &str {
        self.idx = (self.idx + 1) % self.frames.len();
        self.frames[self.idx]
    }
}

#[derive(PartialEq)]
pub enum StatusLine {
    Empty,
    Loading,
    // severity: "error" | "info"
    Text { content: String, severity: String },
}

impl StatusLine {
    pub fn is_empty(&self) -> bool {
        *self == Self::Empty
    }

    pub fn is_loading(&self) -> bool {
        *self == Self::Loading
    }

    pub fn set(&mut self, msg: &str, severity: &str) {
        *self = StatusLine::Text {
            content: msg.to_string(),
            severity: severity.to_string(),
        }
    }

    pub fn clear(&mut self) {
        *self = StatusLine::Empty;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            open_url: None,
            route: Route::Notifications,
            should_quit: false,
            is_loading: false,
            statusline: StatusLine::Empty,
            spinner: Spinner::new(),
            selected_notification_index: 0,
        }
    }
}

impl App {
    pub fn new(event_tx: Sender<NotifEvent>) -> Self {
        Self {
            github: GitHub::new(),
            state: AppState::default(),
            event_tx,
        }
    }

    pub fn dispatch_event(&mut self, event: NotifEvent) -> std::result::Result<(), String> {
        // Will be set to false after async request is completed
        self.state.is_loading = true;
        self.event_tx
            .send(event)
            .map_err(|_| "Could not communicate with network thread".to_string())
    }

    pub fn on_tick(&mut self) -> std::result::Result<(), String> {
        // show loading text but do no overwrite a previous message
        if self.state.is_loading && self.state.statusline.is_empty() {
            self.state.statusline = StatusLine::Loading;
        }

        if !self.state.is_loading && self.state.statusline.is_loading() {
            self.state.statusline.clear();
        }

        // Ensure cursor is always on a notification
        self.state.selected_notification_index = self
            .state
            .selected_notification_index
            .min(self.github.notif.len().saturating_sub(1));

        if let Some(url) = self.state.open_url.take() {
            open::that(url.as_str()).map_err(|_| "Could not open a browser")?;
        }

        Ok(())
    }

    pub fn on_enter(&mut self) -> std::result::Result<(), String> {
        crate::actions::open(self)
    }

    pub fn on_key(&mut self, key: char) -> std::result::Result<(), String> {
        use crate::actions;
        match key {
            'q' => actions::quit(self),
            'o' => actions::open_in_browser(self),
            'd' => actions::mark_as_read(self),
            'R' => actions::refresh(self),
            'g' => actions::goto_begin(self),
            'G' => actions::goto_end(self),
            'j' => actions::next_item(self),
            'k' => actions::previous_item(self),
            _ => Ok(()),
        }
    }
}
