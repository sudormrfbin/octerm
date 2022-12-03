use meow::{
    components::{
        border::{Border, BorderStyle},
        container::Container,
        empty::Empty,
        line::Line,
        padding::Padding,
        Layout, Renderable,
    },
    spans,
    style::{Color, Style, Stylize},
};

use crate::{
    github::{
        self,
        events::{Event, Label},
        User,
    },
    markdown::Markdown,
    util::Boxed,
};

pub struct EventTimeline {
    events: Layout<'static>,
}

impl EventTimeline {
    pub fn new(events: impl IntoIterator<Item = Event>) -> Self {
        let mut layout = Layout::vertical();
        let mut saw_merged_event = false;

        for event in events {
            let renderable: Box<dyn Renderable> = match event {
                Event::Commented(comment) => Comment::from(comment).boxed(),
                Event::Unknown => "Unknown event".bg(Color::Red).fg(Color::Black).boxed(),
                Event::Merged { actor } => {
                    saw_merged_event = true;

                    format!("  Merged by {actor} ")
                        .bg(Color::Purple)
                        .fg(Color::Black)
                        .boxed()
                }
                // Merge events seem to be followed by a redundant closed
                // event, so filter it out if it's already merged.
                Event::Closed { .. } if saw_merged_event => Empty.boxed(),
                Event::Closed { actor } => format!("  Closed by {actor} ")
                    .bg(Color::Red)
                    .fg(Color::Black)
                    .boxed(),
                Event::Committed { message } => {
                    let summary = message.lines().next().unwrap_or_default();
                    format!("  {summary} ").boxed()
                }
                Event::Labeled {
                    actor,
                    label: Label { name, .. },
                } => spans![
                    "  ",
                    actor.to_string(),
                    " added ",
                    name.bold(true),
                    " label"
                ]
                .boxed(),
            };

            layout.push(renderable).push(Line::horizontal().blank());
        }

        Self { events: layout }
    }
}

impl Renderable for EventTimeline {
    fn render(&self, surface: &mut meow::Surface) {
        self.events.render(surface)
    }

    fn size(&self) -> (meow::components::Width, meow::components::Height) {
        self.events.size()
    }
}

pub struct Comment {
    body: Layout<'static>,
}

impl Comment {
    pub fn new(body: String, author: User) -> Self {
        let mut layout = Layout::vertical();
        let header_bg = Color::Blue;
        let header_fg = Color::Black;
        layout
            .push(
                Container::new(format!(" {}", author).bold(true))
                    .bg(header_bg)
                    .fg(header_fg),
            )
            .push(
                Border::new(Padding::new(Markdown::new(body.into())).top(1))
                    .top(false)
                    .style(BorderStyle {
                        style: Style::new().fg(header_bg),
                        ..BorderStyle::outer_edge_aligned()
                    }),
            );
        Self { body: layout }
    }
}

impl From<github::events::Comment> for Comment {
    fn from(c: github::events::Comment) -> Self {
        Self::new(
            c.body.unwrap_or_else(|| "No description provided.".into()),
            c.author,
        )
    }
}

impl Renderable for Comment {
    fn render(&self, surface: &mut meow::Surface) {
        self.body.render(surface)
    }

    fn size(&self) -> (meow::components::Width, meow::components::Height) {
        self.body.size()
    }
}
