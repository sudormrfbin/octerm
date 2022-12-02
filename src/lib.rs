pub mod components;
pub mod error;
pub mod github;
pub mod markdown;
pub mod network;
pub mod util;

use components::{release::ReleaseViewMsg, IssueViewMsg, NotificationsView, NotificationsViewMsg, PullRequestViewMsg};
use github::{Issue, IssueMeta, PullRequestMeta, PullRequest};
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
    PullRequestViewMsg(PullRequestViewMsg),
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
    PullRequest(components::PullRequestView),
    Release(components::ReleaseView),
}

pub enum ServerRequest {
    RefreshNotifs,
    OpenNotifInBrowser(Notification),
    MarkNotifAsRead(Notification),
    OpenIssue(IssueMeta),
    OpenPullRequest(PullRequestMeta),
}

pub enum ServerResponse {
    Notifications(Vec<Notification>),
    MarkedNotifAsRead(Notification),
    Issue(Issue),
    PullRequest(PullRequest),
    AsyncTaskStart,
    AsyncTaskDone,
    Error(Error),
}

pub struct Model {
    notifs: NotificationsView,
    async_task_doing: bool,
    route: Route,
    error: Option<Error>,
}

impl Model {
    fn update_notif_view_msg(&mut self, msg: NotificationsViewMsg) -> Cmd<ServerRequest> {
        match msg {
            NotificationsViewMsg::Refresh => ServerRequest::RefreshNotifs.into(),
            NotificationsViewMsg::OpenInBrowser => {
                ServerRequest::OpenNotifInBrowser(self.notifs.selected().clone()).into()
            }
            NotificationsViewMsg::MarkAsRead => {
                ServerRequest::MarkNotifAsRead(self.notifs.selected().clone()).into()
            }
            NotificationsViewMsg::Open => {
                let notif = self.notifs.selected();
                match notif.target {
                    github::NotificationTarget::Issue(ref meta) => {
                        ServerRequest::OpenIssue(meta.clone()).into()
                    }
                    github::NotificationTarget::PullRequest(ref meta) => {
                        ServerRequest::OpenPullRequest(meta.clone()).into()
                    }
                    github::NotificationTarget::Release(ref release) => {
                        self.route = Route::Release(release.clone().into());
                        Cmd::None
                    }
                    _ => Cmd::None,
                }
            }
            NotificationsViewMsg::CloseView => Cmd::Quit,
            _ => self.notifs.update(msg),
        }
    }

    fn update_issue_view_msg(&mut self, msg: IssueViewMsg) -> Cmd<ServerRequest> {
        match msg {
            IssueViewMsg::CloseView => {
                self.route = Route::Notifications;
                Cmd::None
            }
            IssueViewMsg::OpenInBrowser => {
                // HACK: Ideally we want to open the issue using the issue number
                // stored in the IssueView model instead of relying on the state
                // of another component that is not even in view. But since we
                // don't have a model and only reuse an IssueView, this is a hack.
                ServerRequest::OpenNotifInBrowser(self.notifs.selected().clone()).into()
            }
            _ => match self.route {
                Route::Issue(ref mut issue) => issue.update(msg),
                _ => Cmd::None,
            },
        }
    }

    fn update_pr_view_msg(&mut self, msg: PullRequestViewMsg) -> Cmd<ServerRequest> {
        match msg {
            PullRequestViewMsg::CloseView => {
                self.route = Route::Notifications;
                Cmd::None
            }
            PullRequestViewMsg::OpenInBrowser => {
                ServerRequest::OpenNotifInBrowser(self.notifs.selected().clone()).into()
            }
            _ => match self.route {
                Route::PullRequest(ref mut pr) => pr.update(msg),
                _ => Cmd::None,
            },
        }
    }
    fn update_release_view_msg(&mut self, msg: ReleaseViewMsg) -> Cmd<ServerRequest> {
        match msg {
            ReleaseViewMsg::CloseView => self.route = Route::Notifications,
            ReleaseViewMsg::OpenInBrowser => {
                return ServerRequest::OpenNotifInBrowser(self.notifs.selected().clone()).into()
            }
            _ => {
                if let Route::Release(ref mut release) = self.route {
                    return release.update(msg);
                }
            }
        }

        Cmd::None
    }

    fn update_in_response(&mut self, resp: ServerResponse) -> Cmd<ServerRequest> {
        match resp {
            ServerResponse::Notifications(mut notifs) => {
                self.notifs.list.value_mut(|list| {
                    list.clear();
                    list.append(&mut notifs);
                    meow::components::ListStateSync::Reset
                });
            }
            ServerResponse::MarkedNotifAsRead(n) => self.notifs.list.value_mut(|list| {
                let pos = list.iter().position(|no| no == &n);
                if let Some(pos) = pos {
                    list.remove(pos);
                }

                meow::components::ListStateSync::Adjust
            }),
            ServerResponse::AsyncTaskStart => self.async_task_doing = true,
            ServerResponse::AsyncTaskDone => self.async_task_doing = false,
            ServerResponse::Error(err) => self.error = Some(err),
            ServerResponse::Issue(issue) => {
                self.route = Route::Issue(issue.into());
            }
            ServerResponse::PullRequest(pr) => {
                self.route = Route::PullRequest(pr.into());
            }
        }
        Cmd::None
    }
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
                Route::Notifications => model.notifs.event_to_msg(event).map(Msg::NotifViewMsg),
                Route::Issue(ref issue) => issue.event_to_msg(event).map(Msg::IssueViewMsg),
                Route::Release(ref release) => release.event_to_msg(event).map(Msg::ReleaseViewMsg),
                Route::PullRequest(ref pr) => pr.event_to_msg(event).map(Msg::PullRequestViewMsg),
            },
        }
    }

    fn update(msg: Self::Msg, model: &mut Self::Model) -> meow::Cmd<Self::Request> {
        match msg {
            Msg::ClearError => {
                model.error = None;
                Cmd::None
            }

            Msg::NotifViewMsg(msg) => model.update_notif_view_msg(msg),
            Msg::IssueViewMsg(msg) => model.update_issue_view_msg(msg),
            Msg::ReleaseViewMsg(msg) => model.update_release_view_msg(msg),
            Msg::PullRequestViewMsg(msg) => model.update_pr_view_msg(msg),
            Msg::ServerResponse(resp) => model.update_in_response(resp),
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
            Route::PullRequest(ref pr) => column.push(pr),
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
