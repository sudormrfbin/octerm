pub mod components;
pub mod error;
pub mod github;
pub mod markdown;
pub mod network;
pub mod util;

use components::{
    release::ReleaseViewMsg, IssueViewMsg, NotificationsView, NotificationsViewMsg,
    PullRequestViewMsg,
};
use github::{Issue, IssueMeta, NotificationTarget, PullRequest, PullRequestMeta};
use meow::{
    components::{container::Container, empty::Empty, line::Line, Component, Layout, ListMsg},
    key,
    layout::Constraint,
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
    route: Option<Route>,
    error: Option<Error>,
}

impl Model {
    fn update_notif_view_msg(&mut self, msg: NotificationsViewMsg) -> Cmd<ServerRequest> {
        let mut open_notif = |notifs: &mut NotificationsView| {
            let notif = notifs.selected();
            match notif.target {
                github::NotificationTarget::Issue(ref meta) => {
                    ServerRequest::OpenIssue(meta.clone()).into()
                }
                github::NotificationTarget::PullRequest(ref meta) => {
                    ServerRequest::OpenPullRequest(meta.clone()).into()
                }
                github::NotificationTarget::Release(ref release) => {
                    self.route = Some(Route::Release(release.clone().into()));
                    Cmd::None
                }
                _ => Cmd::None,
            }
        };

        match msg {
            NotificationsViewMsg::Refresh => ServerRequest::RefreshNotifs.into(),
            NotificationsViewMsg::OpenInBrowser => {
                ServerRequest::OpenNotifInBrowser(self.notifs.selected().clone()).into()
            }
            NotificationsViewMsg::MarkAsRead => {
                ServerRequest::MarkNotifAsRead(self.notifs.selected().clone()).into()
            }
            NotificationsViewMsg::Open => open_notif(&mut self.notifs),
            NotificationsViewMsg::CloseView => Cmd::Quit,
            NotificationsViewMsg::OpenNext => {
                self.notifs.list.update::<ServerRequest>(ListMsg::NextItem);
                open_notif(&mut self.notifs)
            }
            NotificationsViewMsg::OpenPrevious => {
                self.notifs.list.update::<ServerRequest>(ListMsg::PrevItem);
                open_notif(&mut self.notifs)
            }
            _ => self.notifs.update(msg),
        }
    }

    fn update_issue_view_msg(&mut self, msg: IssueViewMsg) -> Cmd<ServerRequest> {
        match msg {
            IssueViewMsg::CloseView => {
                self.route = None;
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
                Some(Route::Issue(ref mut issue)) => issue.update(msg),
                _ => Cmd::None,
            },
        }
    }

    fn update_pr_view_msg(&mut self, msg: PullRequestViewMsg) -> Cmd<ServerRequest> {
        match msg {
            PullRequestViewMsg::CloseView => {
                self.route = None;
                Cmd::None
            }
            PullRequestViewMsg::OpenInBrowser => {
                ServerRequest::OpenNotifInBrowser(self.notifs.selected().clone()).into()
            }
            _ => match self.route {
                Some(Route::PullRequest(ref mut pr)) => pr.update(msg),
                _ => Cmd::None,
            },
        }
    }

    fn update_release_view_msg(&mut self, msg: ReleaseViewMsg) -> Cmd<ServerRequest> {
        match msg {
            ReleaseViewMsg::CloseView => self.route = None,
            ReleaseViewMsg::OpenInBrowser => {
                return ServerRequest::OpenNotifInBrowser(self.notifs.selected().clone()).into()
            }
            _ => {
                if let Some(Route::Release(ref mut release)) = self.route {
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
                self.route = Some(Route::Issue(issue.into()));
            }
            ServerResponse::PullRequest(pr) => {
                self.route = Some(Route::PullRequest(pr.into()));
            }
        }
        Cmd::None
    }
}

impl Model {
    fn statusline(&self) -> Layout<'_> {
        let mut statusline = Layout::horizontal();

        if let Some(ref err) = self.error {
            statusline.push(err.to_string().fg(Color::Red));
            return statusline;
        }

        match self.route {
            None => {
                let header = format!(" Notifications • {} ", self.notifs.list.value().len())
                    .bg(Color::Blue)
                    .fg(Color::Black);
                statusline.push(header);
            }
            Some(Route::Issue(_)) => {
                if let Notification {
                    target: target @ NotificationTarget::Issue(issue),
                    ..
                } = self.notifs.selected()
                {
                    let header = format!(
                        " {icon} #{number} ",
                        icon = issue.icon(),
                        number = issue.number
                    )
                    .bg(util::notif_target_color(target))
                    .fg(Color::Black);
                    statusline.push(header).push(" ").push(&issue.title);
                }
            }
            Some(Route::PullRequest(_)) => {
                if let Notification {
                    target: target @ NotificationTarget::PullRequest(pr),
                    ..
                } = self.notifs.selected()
                {
                    let header =
                        format!(" {icon} #{number} ", icon = pr.icon(), number = pr.number)
                            .bg(util::notif_target_color(target))
                            .fg(Color::Black);
                    statusline.push(header).push(" ").push(&pr.title);
                }
            }
            Some(Route::Release(_)) => {
                if let Notification {
                    target: target @ NotificationTarget::Release(rel),
                    ..
                } = self.notifs.selected()
                {
                    let header = format!(" {icon} #{tag} ", icon = rel.icon(), tag = rel.tag_name)
                        .bg(util::notif_target_color(target))
                        .fg(Color::Black);
                    statusline.push(header);
                }
            }
        };

        if self.async_task_doing {
            statusline
                .push_constrained(Empty, Constraint::weak().gte().length(0))
                .push_constrained("Loading...", Constraint::strong().eq().length(10));
        }

        statusline
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
            route: None,
            error: None,
        }
    }

    fn event_to_msg(event: meow::AppEvent, model: &Self::Model) -> Option<Self::Msg> {
        match event {
            key!(Escape) => Some(Msg::ClearError),
            key!(']') => Some(Msg::NotifViewMsg(NotificationsViewMsg::OpenNext)),
            key!('[') => Some(Msg::NotifViewMsg(NotificationsViewMsg::OpenPrevious)),
            key!('d') => Some(Msg::NotifViewMsg(NotificationsViewMsg::MarkAsRead)),
            _ => match model.route {
                None => model.notifs.event_to_msg(event).map(Msg::NotifViewMsg),
                Some(Route::Issue(ref issue)) => issue.event_to_msg(event).map(Msg::IssueViewMsg),
                Some(Route::Release(ref release)) => {
                    release.event_to_msg(event).map(Msg::ReleaseViewMsg)
                }
                Some(Route::PullRequest(ref pr)) => {
                    pr.event_to_msg(event).map(Msg::PullRequestViewMsg)
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

            Msg::NotifViewMsg(msg) => model.update_notif_view_msg(msg),
            Msg::IssueViewMsg(msg) => model.update_issue_view_msg(msg),
            Msg::ReleaseViewMsg(msg) => model.update_release_view_msg(msg),
            Msg::PullRequestViewMsg(msg) => model.update_pr_view_msg(msg),
            Msg::ServerResponse(resp) => model.update_in_response(resp),
        }
    }

    fn view<'m>(model: &'m Self::Model) -> Box<dyn meow::components::Renderable + 'm> {
        let mut notif_view = Layout::horizontal();
        notif_view
            .push_constrained(&model.notifs, Constraint::medium().eq().ratio(1, 2))
            .push_constrained(Line::vertical(), Constraint::strong().eq().length(1));

        match model.route {
            Some(Route::Issue(ref issue)) => notif_view.push(issue),
            Some(Route::Release(ref release)) => notif_view.push(release),
            Some(Route::PullRequest(ref pr)) => notif_view.push(pr),
            None => notif_view.push(Empty),
        }
        .constrain(Constraint::medium().eq().ratio(1, 2));

        let statusline = Container::new(model.statusline())
            .bg(Color::BrightWhite)
            .fg(Color::Black);

        let mut main_view = Layout::vertical();
        main_view
            .push(notif_view)
            .push_constrained(
                Line::horizontal().blank(),
                Constraint::weak().gte().length(0),
            )
            .push_constrained(statusline, Constraint::strong().eq().length(1));

        Box::new(main_view)
    }
}
