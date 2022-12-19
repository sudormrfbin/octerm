use meow::{
    components::{Component, List, ListMsg, Renderable, Span, SizeArgs},
    key, spans,
    style::{Color, Style},
};

use crate::{github, util::notif_target_color};

pub enum NotificationsViewMsg {
    CloseView,
    Open,
    OpenInBrowser,
    Refresh,
    MarkAsRead,

    ListMsg(ListMsg),
}

pub struct NotificationsView {
    pub list: List<github::Notification>,
}

impl NotificationsView {
    pub fn new() -> Self {
        Self {
            list: List::new(Vec::new(), |notif| {
                let icon = notif.target.icon();
                let type_color = notif_target_color(&notif.target);

                let mut type_style = Style::default().fg(type_color);
                let mut repo_style = Style::default();

                if !notif.inner.unread {
                    type_style = type_style.fg(Color::Gray);
                    repo_style = repo_style.fg(Color::Gray);
                }

                let title = notif.inner.subject.title.as_str();
                let repo = notif.inner.repository.name.clone();
                Box::new(spans![
                    Span::new(format!("{repo}: ")).style(repo_style),
                    Span::from(format!("{icon} {title}")).style(type_style),
                ])
            }),
        }
    }

    pub fn selected(&self) -> &github::Notification {
        self.list.selected_item()
    }
}

impl Renderable for NotificationsView {
    fn render(&self, surface: &mut meow::Surface) {
        self.list.render(surface)
    }

    fn size(&self, args: SizeArgs) -> (meow::components::Width, meow::components::Height) {
        self.list.size(args)
    }
}

impl Component for NotificationsView {
    type Msg = NotificationsViewMsg;

    fn event_to_msg(&self, event: meow::AppEvent) -> Option<Self::Msg> {
        match event {
            key!('q') => Some(NotificationsViewMsg::CloseView),
            key!('o') => Some(NotificationsViewMsg::OpenInBrowser),
            key!('R') => Some(NotificationsViewMsg::Refresh),
            key!('d') => Some(NotificationsViewMsg::MarkAsRead),
            key!(Enter) => Some(NotificationsViewMsg::Open),
            _ => self.list.event_to_msg(event).map(Self::Msg::ListMsg),
        }
    }

    fn update<Request>(&mut self, msg: Self::Msg) -> meow::Cmd<Request> {
        match msg {
            NotificationsViewMsg::ListMsg(msg) => self.list.update(msg),
            _ => meow::Cmd::None,
        }
    }
}
