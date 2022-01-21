use std::sync::mpsc::Sender;

use crate::{events::NotifEvent, github::GitHub};

pub struct App {
    pub github: GitHub,
    pub state: AppState,
    pub event_tx: Sender<NotifEvent>,
}

pub struct AppState {
    pub open_url: Option<String>,
    pub should_quit: bool,
    pub is_loading: bool,
    pub status_message: Option<(String, String)>,
    pub selected_notification_index: usize,
}

impl AppState {
    pub fn set_status(&mut self, msg: &str, status: &str) {
        self.status_message = Some((msg.to_string(), status.to_string()));
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn get_status(&mut self) -> Option<&str> {
        self.status_message.as_ref().map(|(msg, _)| msg.as_str())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            open_url: None,
            should_quit: false,
            is_loading: false,
            status_message: None,
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
        const LOADING_DISPLAY: &str = "Loading...";

        if self.state.is_loading && self.state.status_message.is_none() {
            self.state.set_status(LOADING_DISPLAY, "info");
        }

        if !self.state.is_loading && self.state.get_status() == Some(LOADING_DISPLAY) {
            self.state.clear_status();
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
        crate::keybind::actions::open_in_browser(self)
    }

    pub fn on_key(&mut self, key: char) -> std::result::Result<(), String> {
        use crate::keybind::actions;
        match key {
            'q' => actions::quit(self),
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
