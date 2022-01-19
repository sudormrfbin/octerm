pub mod actions {
    use std::ops::Add;

    use crate::app::App;

    pub fn quit(app: &mut App) -> Result<(), String> {
        app.state.should_quit = true;
        Ok(())
    }

    // TODO: This only marks as read, not done; i.e. it will be showed grayed
    // out in the web ui instead of being removed completely. The API currently
    // provides no way to mark as done.
    pub fn mark_as_read(app: &mut App) -> Result<(), String> {
        let notif = app
            .github
            .notif
            .nth(app.state.selected_notification_index)
            .ok_or("Failed to get the current notification")?
            .clone();
        app.github
            .notif
            .mark_as_read(&notif)
            .map_err(|_| "Failed to mark notification as read")?;
        // If last item is deleted, cursor has to moved to previous line
        app.state.selected_notification_index = app
            .state
            .selected_notification_index
            .min(app.github.notif.len().saturating_sub(1));
        Ok(())
    }

    pub fn open_in_browser(app: &mut App) -> Result<(), String> {
        let notif = app
            .github
            .notif
            .nth(app.state.selected_notification_index)
            .ok_or("Failed to get the current notification")?
            .clone();
        let url = app
            .github
            .notif
            .open(&notif)
            .map_err(|_| "Failed to get notification target url")?;
        open::that(url.as_str()).map_err(|_| "Could not open a browser")?;
        Ok(())
    }

    pub fn refresh(app: &mut App) -> Result<(), String> {
        app.github
            .notif
            .refresh()
            .map_err(|_| "Failed to refresh")?;
        app.state.selected_notification_index = 0;
        Ok(())
    }

    pub fn goto_begin(app: &mut App) -> Result<(), String> {
        app.state.selected_notification_index = 0;
        Ok(())
    }

    pub fn goto_end(app: &mut App) -> Result<(), String> {
        app.state.selected_notification_index = app.github.notif.len().saturating_sub(1);
        Ok(())
    }

    pub fn next_item(app: &mut App) -> Result<(), String> {
        app.state.selected_notification_index = app
            .state
            .selected_notification_index
            .add(1)
            .min(app.github.notif.len().saturating_sub(1));
        Ok(())
    }

    pub fn previous_item(app: &mut App) -> Result<(), String> {
        app.state.selected_notification_index =
            app.state.selected_notification_index.saturating_sub(1);
        Ok(())
    }
}
