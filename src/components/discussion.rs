use meow::{
    components::{
        container::Container,
        line::Line,
        padding::Padding,
        scroll::{Scroll, ScrollMsg},
        Component, Layout, Renderable,
    },
    key, spans,
    style::{Color, Stylize},
};

use crate::github;

use super::events::Comment;

pub enum DiscussionViewMsg {
    Scroll(ScrollMsg),
    CloseView,
}

pub struct DiscussionView {
    body: Scroll<Layout<'static>>,
}

impl From<github::Discussion> for DiscussionView {
    fn from(disc: github::Discussion) -> Self {
        let number = format!("#{}", disc.meta.number).fg(Color::Gray);
        let state = match disc.meta.state {
            github::DiscussionState::Answered => " Answered ".bg(Color::Green).fg(Color::Black),
            github::DiscussionState::Unanswered => " Unanswered ".bg(Color::White).fg(Color::Black),
        };
        let mut layout = Layout::vertical().scrollable(true);

        layout
            .push(spans![state, " ", disc.meta.title, " ", number])
            .push(Line::horizontal().blank())
            .push(Comment::new(disc.body, disc.author, disc.created_at.into()));

        for answer in disc.suggested_answers {
            layout
                .push(Line::horizontal().blank())
                .push(DiscussionSuggestedAnswerView::from(answer));
        }

        Self {
            body: Scroll::new(layout),
        }
    }
}

impl Renderable for DiscussionView {
    fn render(&self, surface: &mut meow::Surface) {
        self.body.render(surface)
    }

    fn size(
        &self,
        args: meow::components::SizeArgs,
    ) -> (meow::components::Width, meow::components::Height) {
        self.body.size(args)
    }
}

impl Component for DiscussionView {
    type Msg = DiscussionViewMsg;

    fn event_to_msg(&self, event: meow::AppEvent) -> Option<Self::Msg> {
        match event {
            key!('q') => Some(DiscussionViewMsg::CloseView),
            _ => self.body.event_to_msg(event).map(DiscussionViewMsg::Scroll),
        }
    }

    fn update<Request>(&mut self, msg: Self::Msg) -> meow::Cmd<Request> {
        match msg {
            DiscussionViewMsg::Scroll(msg) => self.body.update(msg),
            _ => meow::Cmd::None,
        }
    }
}

pub struct DiscussionSuggestedAnswerView {
    body: Layout<'static>,
}

impl From<github::DiscussionSuggestedAnswer> for DiscussionSuggestedAnswerView {
    fn from(answer: github::DiscussionSuggestedAnswer) -> Self {
        let mut layout = Layout::vertical();

        if answer.is_answer {
            layout.push(
                Container::new(" Marked as answer ")
                    .bg(Color::Green)
                    .fg(Color::Black),
            );
        }

        layout.push(Comment::new(
            answer.body,
            answer.author,
            answer.created_at.into(),
        ));

        for reply in answer.replies {
            let rendered_reply =
                Comment::new(reply.body, reply.author.into(), reply.created_at.into());
            layout.push(Padding::new(rendered_reply).left(4));
        }

        Self { body: layout }
    }
}

impl Renderable for DiscussionSuggestedAnswerView {
    fn render(&self, surface: &mut meow::Surface) {
        self.body.render(surface)
    }

    fn size(
        &self,
        args: meow::components::SizeArgs,
    ) -> (meow::components::Width, meow::components::Height) {
        self.body.size(args)
    }
}
