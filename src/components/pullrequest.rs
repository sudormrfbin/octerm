use meow::{
    components::{
        line::Line,
        scroll::{Scroll, ScrollMsg},
        Component, Layout, Renderable,
    },
    key, spans,
    style::{Color, Stylize},
};

use crate::github::{
    self,
    events::{self, EventKind},
};

use super::events::EventTimeline;

pub enum PullRequestViewMsg {
    Scroll(ScrollMsg),
    OpenInBrowser,
    CloseView,
}

pub struct PullRequestView {
    body: Scroll<Layout<'static>>,
}

impl From<github::PullRequest> for PullRequestView {
    fn from(pr: github::PullRequest) -> Self {
        let number = format!("#{}", pr.meta.number).fg(Color::Gray);
        let state = match pr.meta.state {
            github::PullRequestState::Open => " Open ".bg(Color::Green),
            github::PullRequestState::Closed => " Closed ".bg(Color::Red),
            github::PullRequestState::Merged => " Merged ".bg(Color::Purple),
        }
        .fg(Color::Black);

        let mut layout = Layout::vertical().scrollable(true);
        layout
            .push(spans![state, " ", pr.meta.title, " ", number])
            .push(Line::horizontal().blank())
            .push(EventTimeline::new(
                std::iter::once(EventKind::Commented(events::Comment {
                    author: pr.meta.author,
                    body: pr.meta.body,
                }))
                .chain(pr.events),
            ));

        Self {
            body: Scroll::new(layout),
        }
    }
}

impl Renderable for PullRequestView {
    fn render(&self, surface: &mut meow::Surface) {
        self.body.render(surface)
    }

    fn size(&self) -> (meow::components::Width, meow::components::Height) {
        self.body.size()
    }
}

impl Component for PullRequestView {
    type Msg = PullRequestViewMsg;

    fn event_to_msg(&self, event: meow::AppEvent) -> Option<Self::Msg> {
        match event {
            key!('q') => Some(PullRequestViewMsg::CloseView),
            key!('o') => Some(PullRequestViewMsg::OpenInBrowser),
            _ => self
                .body
                .event_to_msg(event)
                .map(PullRequestViewMsg::Scroll),
        }
    }

    fn update<Request>(&mut self, msg: Self::Msg) -> meow::Cmd<Request> {
        match msg {
            PullRequestViewMsg::Scroll(msg) => self.body.update(msg),
            _ => meow::Cmd::None,
        }
    }
}
