use crate::github::{
    DiscussionState, IssueClosedReason, IssueState, NotificationTarget, PullRequestState,
};

pub enum NotifColor {
    Purple,
    Green,
    Red,
    White,
    Yellow,
    Blue,
}

pub fn notif_target_color(target: &NotificationTarget) -> NotifColor {
    match target {
        NotificationTarget::Issue(ref issue) => match issue.state {
            IssueState::Open => NotifColor::Green,
            IssueState::Closed(IssueClosedReason::NotPlanned) => NotifColor::Red,
            IssueState::Closed(IssueClosedReason::Completed) => NotifColor::Purple,
        },
        NotificationTarget::PullRequest(ref pr) => match pr.state {
            PullRequestState::Open => NotifColor::Green,
            PullRequestState::Merged => NotifColor::Purple,
            PullRequestState::Closed => NotifColor::Red,
        },
        NotificationTarget::CiBuild => NotifColor::Red,
        NotificationTarget::Release(_) => NotifColor::Blue,
        NotificationTarget::Discussion(ref discussion) => match discussion.state {
            DiscussionState::Unanswered => NotifColor::Yellow,
            DiscussionState::Answered => NotifColor::Purple,
        },
        NotificationTarget::Unknown => NotifColor::White,
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
