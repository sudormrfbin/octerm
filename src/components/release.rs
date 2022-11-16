use crate::markdown::Markdown;
use meow::components::{Component, Renderable};
use meow::style::Stylize;
use meow::{
    column,
    components::{
        line::Line,
        scroll::{Scroll, ScrollMsg},
        Layout,
    },
    spans,
};
use meow::{key, Cmd};

use crate::github;

pub enum ReleaseViewMsg {
    Scroll(ScrollMsg),
    OpenInBrowser,
    CloseView,
}

pub struct ReleaseView {
    body: Scroll<Layout<'static>>,
}

impl From<github::ReleaseMeta> for ReleaseView {
    fn from(release: github::ReleaseMeta) -> Self {
        let layout = column![
            release.title.bold(true),
            spans!["Released by @", release.author],
            Line::horizontal().blank(),
            Markdown::new(release.body.into()),
        ];

        ReleaseView {
            body: Scroll::new(layout),
        }
    }
}

impl Renderable for ReleaseView {
    fn render(&self, surface: &mut meow::Surface) {
        self.body.render(surface)
    }

    fn size(&self) -> (meow::components::Width, meow::components::Height) {
        self.body.size()
    }
}

impl Component for ReleaseView {
    type Msg = ReleaseViewMsg;

    fn event_to_msg(&self, event: meow::AppEvent) -> Option<Self::Msg> {
        match event {
            key!('q') => Some(ReleaseViewMsg::CloseView),
            key!('o') => Some(ReleaseViewMsg::OpenInBrowser),
            _ => self.body.event_to_msg(event).map(ReleaseViewMsg::Scroll),
        }
    }

    fn update<Request>(&mut self, msg: Self::Msg) -> meow::Cmd<Request> {
        match msg {
            ReleaseViewMsg::Scroll(msg) => self.body.update(msg),
            ReleaseViewMsg::OpenInBrowser => Cmd::None,
            ReleaseViewMsg::CloseView => Cmd::None,
        }
    }
}
