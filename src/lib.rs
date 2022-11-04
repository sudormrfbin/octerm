pub mod error;
pub mod github;
pub mod markdown;
pub mod network;
pub mod util;

use meow::{
    components::{blank::BlankLine, Column, Component, List, ListMsg, Span},
    key,
    layout::Constraint,
    spans,
    style::{Color, Style, Stylize},
    App, Cmd, FromResponse,
};

use crate::{error::Error, github::Notification, util::notif_target_color};

pub enum Msg {
    Quit,
    RefreshNotifs,
    OpenNotifInBrowser,
    MarkNotifAsRead,
    ClearError,
    ServerResponse(ServerResponse),

    ListMsg(ListMsg),
}

impl FromResponse<ServerResponse> for Msg {
    fn from_response(response: ServerResponse) -> Self {
        Msg::ServerResponse(response)
    }
}

pub struct Model {
    notifs: List<Notification>,
    async_task_doing: bool,
    error: Option<Error>,
}

pub enum ServerRequest {
    RefreshNotifs,
    OpenNotifInBrowser(Notification),
    MarkNotifAsRead(Notification),
}

pub enum ServerResponse {
    Notifications(Vec<Notification>),
    MarkedNotifAsRead(Notification),
    AsyncTaskStart,
    AsyncTaskDone,
    Error(Error),
}

pub struct OctermApp {}

impl App for OctermApp {
    type Msg = Msg;
    type Model = Model;

    type Request = ServerRequest;
    type Response = ServerResponse;

    fn init() -> Self::Model {
        Model {
            notifs: List::new(Vec::new(), |notif| {
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
            async_task_doing: false,
            error: None,
        }
    }

    fn event_to_msg(event: meow::AppEvent, _model: &Self::Model) -> Option<Self::Msg> {
        match event {
            key!('q') => Some(Msg::Quit),
            key!('o') => Some(Msg::OpenNotifInBrowser),
            key!('R') => Some(Msg::RefreshNotifs),
            key!('d') => Some(Msg::MarkNotifAsRead),
            key!(Escape) => Some(Msg::ClearError),
            _ => Some(Msg::ListMsg(_model.notifs.event_to_msg(event)?)),
        }
    }

    fn update(msg: Self::Msg, model: &mut Self::Model) -> meow::Cmd<Self::Request> {
        match msg {
            Msg::Quit => Cmd::Quit,
            Msg::ClearError => {
                model.error = None;
                Cmd::None
            }
            Msg::RefreshNotifs => Cmd::ServerRequest(ServerRequest::RefreshNotifs),
            Msg::OpenNotifInBrowser => Cmd::ServerRequest(ServerRequest::OpenNotifInBrowser(
                model.notifs.selected_item().clone(),
            )),
            Msg::MarkNotifAsRead => Cmd::ServerRequest(ServerRequest::MarkNotifAsRead(
                model.notifs.selected_item().clone(),
            )),
            Msg::ListMsg(msg) => model.notifs.update(msg),
            Msg::ServerResponse(resp) => {
                match resp {
                    ServerResponse::Notifications(mut notifs) => {
                        model.notifs.value_mut(|list| {
                            list.clear();
                            list.append(&mut notifs);
                            meow::components::ListStateSync::Reset
                        });
                    }
                    ServerResponse::MarkedNotifAsRead(n) => model.notifs.value_mut(|list| {
                        let pos = list.iter().position(|no| no == &n);
                        if let Some(pos) = pos {
                            list.remove(pos);
                        }

                        meow::components::ListStateSync::Adjust
                    }),
                    ServerResponse::AsyncTaskStart => model.async_task_doing = true,
                    ServerResponse::AsyncTaskDone => model.async_task_doing = false,
                    ServerResponse::Error(err) => model.error = Some(err),
                }
                Cmd::None
            }
        }
    }

    fn view<'m>(model: &'m Self::Model) -> Box<dyn meow::components::Renderable + 'm> {
        let mut column = Column::new();
        let mut status_line = spans![model
            .error
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .fg(Color::Red)];

        if model.async_task_doing {
            status_line.0.insert(0, "Loading... | ".into())
        }

        column
            .push(&model.notifs)
            .push_constrained(BlankLine::SINGLE, Constraint::weak().gte().length(0))
            .push_constrained(status_line, Constraint::strong().eq().length(1));

        Box::new(column)
    }
}
