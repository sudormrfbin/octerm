use meow::{
    components::{
        line::Line,
        scroll::{Scroll, ScrollMsg},
        Component, Layout, Renderable,
    },
    key, spans,
    style::{Color, Stylize},
};

use crate::github::{self, events};

use super::events::EventTimeline;

pub enum IssueViewMsg {
    Scroll(ScrollMsg),
    OpenInBrowser,
    CloseView,
}

pub struct IssueView {
    body: Scroll<Layout<'static>>,
}

impl From<github::Issue> for IssueView {
    fn from(issue: github::Issue) -> Self {
        let number = format!("#{}", issue.meta.number).fg(Color::Gray);
        let state = match issue.meta.state {
            github::IssueState::Open => " Open ".bg(Color::Green),
            github::IssueState::Closed(reason) => " Closed ".bg(match reason {
                github::IssueClosedReason::Completed => Color::Purple,
                github::IssueClosedReason::NotPlanned => Color::Red,
            }),
        }
        .fg(Color::Black);

        let mut layout = Layout::vertical().scrollable(true);
        layout
            .push(spans![state, " ", issue.meta.title, " ", number])
            .push(Line::horizontal().blank())
            .push(EventTimeline::new(
                std::iter::once(
                    events::EventKind::Commented {
                        body: issue.meta.body,
                    }
                    .with(issue.meta.author, issue.meta.created_at),
                )
                .chain(issue.events),
            ));

        Self {
            body: Scroll::new(layout),
        }
    }
}

impl Renderable for IssueView {
    fn render(&self, surface: &mut meow::Surface) {
        self.body.render(surface)
    }

    fn size(&self) -> (meow::components::Width, meow::components::Height) {
        self.body.size()
    }
}

impl Component for IssueView {
    type Msg = IssueViewMsg;

    fn event_to_msg(&self, event: meow::AppEvent) -> Option<Self::Msg> {
        match event {
            key!('q') => Some(IssueViewMsg::CloseView),
            key!('o') => Some(IssueViewMsg::OpenInBrowser),
            _ => self.body.event_to_msg(event).map(IssueViewMsg::Scroll),
        }
    }

    fn update<Request>(&mut self, msg: Self::Msg) -> meow::Cmd<Request> {
        match msg {
            IssueViewMsg::Scroll(msg) => self.body.update(msg),
            _ => meow::Cmd::None,
        }
    }
}
