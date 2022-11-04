use meow::style::Color;

use crate::github::{IssueState, NotificationTarget, PullRequestState};

pub fn notif_target_color(target: &NotificationTarget) -> Color {
    match target {
        NotificationTarget::Issue(ref issue) => match issue.state {
            IssueState::Open => Color::Green,
            IssueState::Closed => Color::Red,
        },
        NotificationTarget::PullRequest(ref pr) => match pr.state {
            PullRequestState::Open => Color::Green,
            PullRequestState::Merged => Color::Magenta,
            PullRequestState::Closed => Color::Red,
        },
        NotificationTarget::CiBuild => Color::Red,
        NotificationTarget::Release(_) => Color::Blue,
        NotificationTarget::Discussion => Color::Yellow,
        NotificationTarget::Unknown => Color::White,
    }
}
