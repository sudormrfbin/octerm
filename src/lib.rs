pub mod components;
pub mod error;
pub mod github;
pub mod markdown;
pub mod network;
pub mod util;

use components::{release::ReleaseViewMsg, IssueViewMsg, NotificationsView, NotificationsViewMsg};
use github::{Issue, IssueMeta};
use meow::{
    components::{line::Line, Component, Layout},
    key,
    layout::Constraint,
    spans,
    style::{Color, Stylize},
    App, Cmd, FromResponse,
};

use crate::{error::Error, github::Notification};

pub enum Msg {
    ClearError,
    ServerResponse(ServerResponse),

    NotifViewMsg(NotificationsViewMsg),
    IssueViewMsg(IssueViewMsg),
    ReleaseViewMsg(ReleaseViewMsg),
}

impl FromResponse<ServerResponse> for Msg {
    fn from_response(response: ServerResponse) -> Self {
        Msg::ServerResponse(response)
    }
}

pub enum Route {
    Notifications,
    Issue(components::IssueView),
    Release(components::ReleaseView),
}

pub struct Model {
    notifs: NotificationsView,
    async_task_doing: bool,
    route: Route,
    error: Option<Error>,
}

pub enum ServerRequest {
    RefreshNotifs,
    OpenNotifInBrowser(Notification),
    MarkNotifAsRead(Notification),
    OpenIssue(IssueMeta),
}

impl From<ServerRequest> for Cmd<ServerRequest> {
    fn from(req: ServerRequest) -> Self {
        Cmd::ServerRequest(req)
    }
}

pub enum ServerResponse {
    Notifications(Vec<Notification>),
    MarkedNotifAsRead(Notification),
    Issue(Issue),
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
            notifs: NotificationsView::new(),
            async_task_doing: false,
            route: Route::Notifications,
            error: None,
        }
    }

    fn event_to_msg(event: meow::AppEvent, model: &Self::Model) -> Option<Self::Msg> {
        match event {
            key!(Escape) => Some(Msg::ClearError),
            _ => match model.route {
                Route::Notifications => Some(Msg::NotifViewMsg(model.notifs.event_to_msg(event)?)),
                Route::Issue(ref issue) => Some(Msg::IssueViewMsg(issue.event_to_msg(event)?)),
                Route::Release(ref release) => {
                    Some(Msg::ReleaseViewMsg(release.event_to_msg(event)?))
                }
            },
        }
    }

    fn update(msg: Self::Msg, model: &mut Self::Model) -> meow::Cmd<Self::Request> {
        match msg {
            Msg::ClearError => {
                model.error = None;
                Cmd::None
            }

            Msg::NotifViewMsg(NotificationsViewMsg::Refresh) => {
                Cmd::ServerRequest(ServerRequest::RefreshNotifs)
            }
            Msg::NotifViewMsg(NotificationsViewMsg::OpenInBrowser) => Cmd::ServerRequest(
                ServerRequest::OpenNotifInBrowser(model.notifs.selected().clone()),
            ),
            Msg::NotifViewMsg(NotificationsViewMsg::MarkAsRead) => Cmd::ServerRequest(
                ServerRequest::MarkNotifAsRead(model.notifs.selected().clone()),
            ),
            Msg::NotifViewMsg(NotificationsViewMsg::Open) => {
                let notif = model.notifs.selected();
                match notif.target {
                    github::NotificationTarget::Issue(ref meta) => {
                        Cmd::ServerRequest(ServerRequest::OpenIssue(meta.clone()))
                    }
                    github::NotificationTarget::Release(ref release) => {
                        model.route = Route::Release(release.clone().into());
                        Cmd::None
                    }
                    _ => Cmd::None,
                }
            }
            Msg::NotifViewMsg(NotificationsViewMsg::CloseView) => Cmd::Quit,
            Msg::NotifViewMsg(msg) => model.notifs.update(msg),

            Msg::IssueViewMsg(IssueViewMsg::CloseView) => {
                model.route = Route::Notifications;
                Cmd::None
            }
            Msg::IssueViewMsg(IssueViewMsg::OpenInBrowser) => Cmd::ServerRequest(
                // HACK: Ideally we want to open the issue using the issue number
                // stored in the IssueView model instead of relying on the state
                // of another component that is not even in view. But since we
                // don't have a model and only reuse an IssueView, this is a hack.
                ServerRequest::OpenNotifInBrowser(model.notifs.selected().clone()),
            ),
            Msg::IssueViewMsg(msg) => match model.route {
                Route::Issue(ref mut issue) => issue.update(msg),
                _ => Cmd::None,
            },
            Msg::ReleaseViewMsg(ReleaseViewMsg::CloseView) => {
                model.route = Route::Notifications;
                Cmd::None
            }
            Msg::ReleaseViewMsg(ReleaseViewMsg::OpenInBrowser) => {
                ServerRequest::OpenNotifInBrowser(model.notifs.selected().clone()).into()
            }
            Msg::ReleaseViewMsg(msg) => match model.route {
                Route::Release(ref mut release) => release.update(msg),
                _ => Cmd::None,
            },

            Msg::ServerResponse(resp) => {
                match resp {
                    ServerResponse::Notifications(mut notifs) => {
                        model.notifs.list.value_mut(|list| {
                            list.clear();
                            list.append(&mut notifs);
                            meow::components::ListStateSync::Reset
                        });
                    }
                    ServerResponse::MarkedNotifAsRead(n) => model.notifs.list.value_mut(|list| {
                        let pos = list.iter().position(|no| no == &n);
                        if let Some(pos) = pos {
                            list.remove(pos);
                        }

                        meow::components::ListStateSync::Adjust
                    }),
                    ServerResponse::AsyncTaskStart => model.async_task_doing = true,
                    ServerResponse::AsyncTaskDone => model.async_task_doing = false,
                    ServerResponse::Error(err) => model.error = Some(err),
                    ServerResponse::Issue(issue) => {
                        model.route = Route::Issue(issue.into());
                    }
                }
                Cmd::None
            }
        }
    }

    fn view<'m>(model: &'m Self::Model) -> Box<dyn meow::components::Renderable + 'm> {
        let mut column = Layout::vertical();
        let mut status_line = spans![model
            .error
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .fg(Color::Red)];

        if model.async_task_doing {
            match status_line.0.len() {
                0 => status_line.0.push("Loading...".into()),
                _ => status_line.0.insert(0, "Loading... | ".into()),
            }
        }

        match model.route {
            Route::Notifications => column.push(&model.notifs),
            Route::Issue(ref issue) => column.push(issue),
            Route::Release(ref release) => column.push(release),
        };

        column
            .push_constrained(
                Line::horizontal().blank(),
                Constraint::weak().gte().length(0),
            )
            .push_constrained(status_line, Constraint::strong().eq().length(1));

        Box::new(column)
    }
}
