use crate::github::Notification;

pub enum NotifEvent {
    Refresh,
    Open(Notification),
    MarkAsRead(Notification),
}
