use meow::{
    components::{
        border::{Border, BorderStyle},
        container::Container,
        line::Line,
        scroll::{Scroll, ScrollMsg},
        Component, Layout, Renderable,
    },
    key, spans,
    style::{Color, Style, Stylize},
};

use crate::{github, markdown::Markdown};

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
                github::IssueClosedReason::Completed => Color::Magenta,
                github::IssueClosedReason::NotPlanned => Color::Red,
            }),
        }
        .fg(Color::Black);

        let mut layout = Layout::vertical().scrollable(true);
        layout
            .push(spans![state, " ", issue.meta.title, " ", number])
            .push(Line::horizontal().blank())
            .push(Container::new(IssueComment::new(
                issue.meta.body.into(),
                issue.meta.author.name,
            )));

        for event in issue.events {
            if let github::events::Event::Unknown = event {
                continue;
            }

            layout
                .push(Line::horizontal().blank())
                .push(Container::new(match event {
                    github::events::Event::Commented(comment) => IssueComment::from(comment),
                    github::events::Event::Unknown => unreachable!(),
                }));
        }

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

pub struct IssueComment {
    body: Layout<'static>,
}

impl IssueComment {
    pub fn new(body: String, author: String) -> Self {
        let mut layout = Layout::vertical();
        let header_bg = Color::Blue;
        let header_fg = Color::Black;
        layout
            .push(
                Container::new(format!(" @{}", author).bold(true))
                    .bg(header_bg)
                    .fg(header_fg),
            )
            .push(
                Border::new(meow::column![
                    Line::horizontal().blank(),
                    Markdown::new(body.into()),
                ])
                .top(false)
                .style(BorderStyle {
                    style: Style::new().fg(header_bg),
                    ..BorderStyle::outer_edge_aligned()
                }),
            );
        Self { body: layout }
    }
}

impl From<github::IssueComment> for IssueComment {
    fn from(c: github::IssueComment) -> Self {
        Self::new(
            c.body.unwrap_or_else(|| "No description provided.".into()),
            c.author.name,
        )
    }
}

impl Renderable for IssueComment {
    fn render(&self, surface: &mut meow::Surface) {
        self.body.render(surface)
    }

    fn size(&self) -> (meow::components::Width, meow::components::Height) {
        self.body.size()
    }
}
