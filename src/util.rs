use meow::style::Color;

use crate::github::{
    DiscussionState, IssueClosedReason, IssueState, NotificationTarget, PullRequestState,
};

pub fn notif_target_color(target: &NotificationTarget) -> Color {
    match target {
        NotificationTarget::Issue(ref issue) => match issue.state {
            IssueState::Open => Color::Green,
            IssueState::Closed(IssueClosedReason::NotPlanned) => Color::Red,
            IssueState::Closed(IssueClosedReason::Completed) => Color::Purple,
        },
        NotificationTarget::PullRequest(ref pr) => match pr.state {
            PullRequestState::Open => Color::Green,
            PullRequestState::Merged => Color::Purple,
            PullRequestState::Closed => Color::Red,
        },
        NotificationTarget::CiBuild => Color::Red,
        NotificationTarget::Release(_) => Color::Blue,
        NotificationTarget::Discussion(ref discussion) => match discussion.state {
            DiscussionState::Unanswered => Color::Yellow,
            DiscussionState::Answered => Color::Green,
        },
        NotificationTarget::Unknown => Color::White,
    }
}

/// Utility trait for writing value.boxed() instead of Box::new(value).
pub trait Boxed {
    fn boxed(self) -> Box<Self>;
}

impl<T> Boxed for T {
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}
