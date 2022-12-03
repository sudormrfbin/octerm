use meow::{
    components::{
        border::{Border, BorderStyle},
        container::Container,
        line::Line,
        padding::Padding,
        text::Text,
        Layout, Renderable,
    },
    spans,
    style::{Color, Style, Stylize},
};

use crate::{
    github::{
        self,
        events::{Event, Label, RenameEvent, ReviewState},
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
                Event::Unknown => "Unknown event".fg(Color::Red).italic(true).boxed(),
                Event::Merged { actor, .. } => {
                    saw_merged_event = true;

                    spans![
                        "  ".fg(Color::Purple),
                        " Merged ".bg(Color::Purple).fg(Color::Black),
                        " by ",
                        actor.to_string()
                    ]
                    .boxed()
                }
                // Merge events seem to be followed by a redundant closed
                // event, so filter it out if it's already merged.
                Event::Closed { .. } if saw_merged_event => continue,
                // TODO: Use correct icon here based on PR/issue
                Event::Closed { actor } => spans![
                    "  ".fg(Color::Red),
                    " Closed ".bg(Color::Red).fg(Color::Black),
                    " by ",
                    actor.to_string()
                ]
                .boxed(),
                Event::Reopened { actor } => spans![
                    "  ".fg(Color::Green),
                    " Reopened ".bg(Color::Green).fg(Color::Black),
                    " by ",
                    actor.to_string(),
                ]
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
                Event::Unlabeled {
                    actor,
                    label: Label { name, .. },
                } => spans![
                    "  ",
                    actor.to_string(),
                    " removed ",
                    name.bold(true),
                    " label"
                ]
                .boxed(),
                Event::MarkedAsDuplicate { actor } => {
                    format!("  {actor} marked this as a duplicate").boxed()
                }
                Event::UnmarkedAsDuplicate { actor } => {
                    format!("  {actor} marked this as not a duplicate").boxed()
                }
                Event::CrossReferenced { actor, source } => Text::new(vec![
                    spans!["  Cross referenced by ", actor.to_string(), " from"],
                    spans![
                        "   ",
                        source.issue.title.underline(meow::style::Underline::Single),
                        format!(" #{}", source.issue.number).fg(Color::Gray)
                    ],
                ])
                .boxed(),
                Event::HeadRefForcePushed { actor } => {
                    format!["  {actor} force-pushed the branch"].boxed()
                }
                Event::HeadRefDeleted { actor } => format!["  {actor} deleted the branch"].boxed(),
                Event::Renamed {
                    actor,
                    rename: RenameEvent { from, to },
                } => Text::new(vec![
                    spans!["  ", actor.to_string(), " changed the title"],
                    spans!["   ", from.strikethrough(true)],
                    spans!["   ", to],
                ])
                .boxed(),
                Event::Reviewed { state, actor, body } => {
                    let state_text = match state {
                        ReviewState::Commented => {
                            spans!(
                                "  ".fg(Color::Gray),
                                actor.to_string(),
                                " ",
                                " reviewed ".bg(Color::Gray).fg(Color::White),
                                " changes "
                            )
                        }
                        ReviewState::Approved => {
                            spans!(
                                "  ".fg(Color::Green),
                                actor.to_string(),
                                " ",
                                " approved ".bg(Color::Green).fg(Color::Black),
                                " these changes "
                            )
                        }
                        ReviewState::ChangesRequested => {
                            spans!(
                                "  ".fg(Color::Red),
                                actor.to_string(),
                                " ",
                                " requested ".bg(Color::Red).fg(Color::Black),
                                " changes "
                            )
                        }
                    };

                    match body.filter(|b| !b.is_empty()) {
                        Some(body) => meow::column![
                            state_text,
                            Line::horizontal().blank(),
                            Comment::new(body, actor),
                        ]
                        .boxed(),
                        None => state_text.boxed(),
                    }
                }
                Event::Connected { actor: _ } => {
                    // TODO: Use correct nouns here (linked an issue/PR to close this issue/PR)
                    // format!("  {actor} linked to another PR/issue").boxed()
                    continue;
                }
                Event::Mentioned | Event::Subscribed => continue,
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
