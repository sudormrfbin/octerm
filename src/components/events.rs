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
        events::{Event, EventKind, Label, ReviewState},
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
            let actor = event.actor;
            let renderable: Box<dyn Renderable> = match event.kind {
                EventKind::Assigned { assignee } => {
                    format!("  {actor} assigned {assignee}").boxed()
                }
                EventKind::Commented { body } => Comment::new(body, actor).boxed(),
                EventKind::Unknown(name) => format!("Unhandled event '{name}'")
                    .fg(Color::Red)
                    .italic(true)
                    .boxed(),
                EventKind::Merged { base_branch } => {
                    saw_merged_event = true;

                    spans![
                        "  ".fg(Color::Purple),
                        " Merged ".bg(Color::Purple).fg(Color::Black),
                        " by ",
                        actor.to_string(),
                        " into ",
                        base_branch,
                    ]
                    .boxed()
                }
                // Merge events seem to be followed by a redundant closed
                // event, so filter it out if it's already merged.
                EventKind::Closed { .. } if saw_merged_event => continue,
                // TODO: Use correct icon here based on PR/issue
                EventKind::Closed { closer } => {
                    let mut spans = spans![
                        "  ".fg(Color::Red),
                        " Closed ".bg(Color::Red).fg(Color::Black),
                        " by ",
                        actor.to_string()
                    ];
                    if let Some(closer) = closer {
                        let end = match closer {
                            github::events::IssueCloser::Commit { abbr_oid } => {
                                format!(" in {abbr_oid}")
                            }
                            github::events::IssueCloser::PullRequest { number } => {
                                format!(" in #{number}")
                            }
                        };
                        spans.0.push(end.into());
                    }
                    spans.boxed()
                }
                EventKind::Reopened {} => spans![
                    "  ".fg(Color::Green),
                    " Reopened ".bg(Color::Green).fg(Color::Black),
                    " by ",
                    actor.to_string(),
                ]
                .boxed(),
                EventKind::Committed {
                    message_headline: message,
                    abbreviated_oid,
                } => spans!["  ", message, " ", abbreviated_oid.fg(Color::Gray)].boxed(),
                EventKind::Labeled {
                    label: Label { name },
                } => spans![
                    "  ",
                    actor.to_string(),
                    " added ",
                    name.bold(true),
                    " label"
                ]
                .boxed(),
                EventKind::Unlabeled {
                    label: Label { name },
                } => spans![
                    "  ",
                    actor.to_string(),
                    " removed ",
                    name.bold(true),
                    " label"
                ]
                .boxed(),
                EventKind::MarkedAsDuplicate { original } => {
                    let (title, number) = original
                        .as_ref()
                        .map(|o| (o.title().to_string(), o.number()))
                        .unwrap_or_default();

                    Text::new(vec![
                        spans!["  ", actor.to_string(), " marked this as a duplicate of"],
                        spans![
                            "   ",
                            title.underline(meow::style::Underline::Single),
                            format!(" #{}", number).fg(Color::Gray)
                        ],
                    ])
                    .boxed()
                }
                EventKind::UnmarkedAsDuplicate {} => {
                    format!("  {actor} marked this as not a duplicate").boxed()
                }
                EventKind::CrossReferenced {
                    source,
                    cross_repository,
                } => {
                    let number = source.number();
                    let title = source.title();
                    let source = match cross_repository {
                        Some(github::events::Repository { name, owner }) => {
                            format!("{owner}/{name}#{number}")
                        }
                        None => format!("#{number}"),
                    };
                    Text::new(vec![
                        spans!["  Cross referenced by ", actor.to_string(), " from"],
                        spans![
                            "   ",
                            title.to_string().underline(meow::style::Underline::Single),
                            " ",
                            source.fg(Color::Gray)
                        ],
                    ])
                    .boxed()
                }
                EventKind::HeadRefForcePushed {
                    before_commit_abbr_oid: before,
                    after_commit_abbr_oid: after,
                } => spans![
                    "  ",
                    actor.to_string(),
                    " force-pushed the branch from ",
                    before.fg(Color::Gray),
                    " to ",
                    after.fg(Color::Gray)
                ]
                .boxed(),
                EventKind::HeadRefDeleted { branch } => {
                    format!["  {actor} deleted the {branch} branch"].boxed()
                }
                EventKind::Renamed { from, to } => Text::new(vec![
                    spans!["  ", actor.to_string(), " changed the title"],
                    spans!["   ", from.strikethrough(true)],
                    spans!["   ", to],
                ])
                .boxed(),
                EventKind::Reviewed { state, body } => {
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
                        _ => spans![],
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
                EventKind::Connected { source } => {
                    // TODO: Use correct nouns here (linked an issue/PR to close this issue/PR)
                    let source_typ = match source {
                        github::events::IssueOrPullRequest::PullRequest { .. } => "pull request",
                        github::events::IssueOrPullRequest::Issue { .. } => "issue",
                    };
                    Text::new(vec![
                        spans![format!(
                            "  {actor} linked a {source_typ} that will close this"
                        )],
                        spans![
                            "   ",
                            source
                                .title()
                                .to_string()
                                .underline(meow::style::Underline::Single),
                            format!(" #{}", source.number()).fg(Color::Gray)
                        ],
                    ])
                    .boxed()
                }
                EventKind::Locked { reason: _ } => {
                    format!("  {actor} locked and limited conversation to collaborators").boxed()
                }
                EventKind::Milestoned { title } => {
                    format!("  {actor} added this to the {title} milestone").boxed()
                }

                EventKind::Pinned {} => format!("  {actor} pinned this").boxed(),
                EventKind::Unpinned {} => format!("  {actor} unpinned this").boxed(),
                EventKind::Unassigned { assignee } => {
                    format!("  {actor} unassigned {assignee}").boxed()
                }
                EventKind::Unlocked {} => format!("  {actor} unlocked this").boxed(),

                EventKind::Referenced {
                    commit_msg_summary,
                    cross_repository,
                } => {
                    let repo_name = match cross_repository {
                        Some(repo) => format!(" to {}/{} ", repo.owner, repo.name),
                        None => " ".to_string(),
                    };
                    let mut text = Vec::new();
                    text.push(
                        format!("  {actor} added a commit{repo_name}that referenced this").into(),
                    );
                    text.push(format!("   {commit_msg_summary}").into());
                    Text::new(text).boxed()
                }
                EventKind::Mentioned | EventKind::Subscribed => continue,

                EventKind::MarkedAsDraft {} => {
                    format!("  {actor} marked this pull request as draft").boxed()
                }
                EventKind::MarkedAsReadyForReview {} => {
                    format!("  {actor} marked this pull request as ready for review").boxed()
                }
                EventKind::ReviewRequested {
                    requested_reviewer: reviewer,
                } => format!("  {actor} requested a review from {reviewer}").boxed(),
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

impl Renderable for Comment {
    fn render(&self, surface: &mut meow::Surface) {
        self.body.render(surface)
    }

    fn size(&self) -> (meow::components::Width, meow::components::Height) {
        self.body.size()
    }
}
