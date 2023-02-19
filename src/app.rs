use crate::components::{
    release::ReleaseViewMsg, DiscussionViewMsg, IssueViewMsg, NotificationsView,
    NotificationsViewMsg, PullRequestViewMsg,
};
use crate::github::{
    Discussion, DiscussionMeta, Issue, IssueMeta, NotificationTarget, PullRequest, PullRequestMeta,
};
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
    DiscussionViewMsg(DiscussionViewMsg),
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
    Discussion(components::DiscussionView),
}

pub enum ServerRequest {
    RefreshNotifs,
    OpenNotifInBrowser(Notification),
    MarkNotifAsRead(Notification),
    OpenIssue(IssueMeta),
    OpenPullRequest(PullRequestMeta),
    OpenDiscussion(DiscussionMeta),
}

pub enum ServerResponse {
    Notifications(Vec<Notification>),
    MarkedNotifAsRead(Notification),
    Issue(Issue),
    PullRequest(PullRequest),
    Discussion(Discussion),
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
        let open_notif = |model: &mut Model| {
            let notif = model.notifs.selected();
            match notif.target {
                github::NotificationTarget::Issue(ref meta) => {
                    ServerRequest::OpenIssue(meta.clone()).into()
                }
                github::NotificationTarget::PullRequest(ref meta) => {
                    ServerRequest::OpenPullRequest(meta.clone()).into()
                }
                github::NotificationTarget::Release(ref release) => {
                    model.route = Some(Route::Release(release.clone().into()));
                    Cmd::None
                }
                github::NotificationTarget::Discussion(ref meta) => {
                    ServerRequest::OpenDiscussion(meta.clone()).into()
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
                let mark_as_read = ServerRequest::MarkNotifAsRead(self.notifs.selected().clone());
                self.notifs.list.update::<ServerRequest>(ListMsg::NextItem);
                self.route = None;
                match open_notif(self) {
                    Cmd::ServerRequest(open) => Cmd::ServerRequests(vec![mark_as_read, open]),
                    _ => mark_as_read.into(),
                }
            }
            NotificationsViewMsg::Open => open_notif(self),
            NotificationsViewMsg::CloseView => Cmd::Quit,
            NotificationsViewMsg::OpenNext => {
                self.notifs.list.update::<ServerRequest>(ListMsg::NextItem);
                open_notif(self)
            }
            NotificationsViewMsg::OpenPrevious => {
                self.notifs.list.update::<ServerRequest>(ListMsg::PrevItem);
                open_notif(self)
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
            _ => match self.route {
                Some(Route::PullRequest(ref mut pr)) => pr.update(msg),
                _ => Cmd::None,
            },
        }
    }

    fn update_release_view_msg(&mut self, msg: ReleaseViewMsg) -> Cmd<ServerRequest> {
        match msg {
            ReleaseViewMsg::CloseView => self.route = None,
            _ => {
                if let Some(Route::Release(ref mut release)) = self.route {
                    return release.update(msg);
                }
            }
        }

        Cmd::None
    }

    fn update_discussion_view_msg(&mut self, msg: DiscussionViewMsg) -> Cmd<ServerRequest> {
        match msg {
            DiscussionViewMsg::CloseView => self.route = None,
            _ => {
                if let Some(Route::Discussion(ref mut disc)) = self.route {
                    return disc.update(msg);
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
            ServerResponse::MarkedNotifAsRead(n) => {
                let mut idx = self.notifs.list.selected_index();

                self.notifs.list.value_mut(|list| {
                    let pos = list.iter().position(|no| no == &n);
                    if let Some(pos) = pos {
                        idx = idx.saturating_sub(1);
                        list.remove(pos);
                    }

                    meow::components::ListStateSync::Adjust
                });

                // Reposition the cursor to the previously selected item.
                self.notifs.list.set_selected_index(idx);
            }
            ServerResponse::AsyncTaskStart => self.async_task_doing = true,
            ServerResponse::AsyncTaskDone => self.async_task_doing = false,
            ServerResponse::Error(err) => self.error = Some(err),
            ServerResponse::Issue(issue) => {
                self.route = Some(Route::Issue(issue.into()));
            }
            ServerResponse::PullRequest(pr) => {
                self.route = Some(Route::PullRequest(pr.into()));
            }
            ServerResponse::Discussion(disc) => {
                self.route = Some(Route::Discussion(disc.into()));
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
            Some(Route::Discussion(_)) => {
                if let Notification {
                    target: target @ NotificationTarget::Discussion(disc),
                    ..
                } = self.notifs.selected()
                {
                    let header = format!(
                        " {icon} #{number} ",
                        icon = disc.icon(),
                        number = disc.number
                    )
                    .bg(util::notif_target_color(target))
                    .fg(Color::Black);
                    statusline.push(header).push(" ").push(&disc.title);
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
        let event_cloned = event.clone();
        match event {
            key!(Escape) => Some(Msg::ClearError),
            _ => match model.route {
                Some(Route::Issue(ref issue)) => issue.event_to_msg(event).map(Msg::IssueViewMsg),
                Some(Route::Release(ref release)) => {
                    release.event_to_msg(event).map(Msg::ReleaseViewMsg)
                }
                Some(Route::PullRequest(ref pr)) => {
                    pr.event_to_msg(event).map(Msg::PullRequestViewMsg)
                }
                Some(Route::Discussion(ref disc)) => {
                    disc.event_to_msg(event).map(Msg::DiscussionViewMsg)
                }
                None => None,
            }
            .or_else(|| {
                model
                    .notifs
                    .event_to_msg(event_cloned)
                    .map(Msg::NotifViewMsg)
            }),
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
            Msg::DiscussionViewMsg(msg) => model.update_discussion_view_msg(msg),
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
            Some(Route::Discussion(ref disc)) => notif_view.push(disc),
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
