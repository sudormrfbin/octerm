use super::{IssueDeserModel, User};

pub enum Event {
    Assigned {
        assignee: User,
        actor: User,
    },
    Commented(Comment),
    Merged {
        actor: User,
        /// The commit that was merged into the main branch
        commit_id: String,
    },
    Closed {
        actor: User,
        /// The issue was closed automatically because a PR/commit was linked
        /// here and was merged/committed.
        closer: Option<IssueCloser>,
    },
    Committed {
        message: String,
    },
    Labeled {
        actor: User,
        label: Label,
    },
    Unlabeled {
        actor: User,
        label: Label,
    },
    MarkedAsDuplicate {
        actor: User,
        original: Option<IssueOrPullRequest>,
    },
    UnmarkedAsDuplicate {
        actor: User,
    },
    CrossReferenced {
        actor: User,
        source: IssueOrPullRequest,
    },
    HeadRefForcePushed {
        actor: User,
    },
    HeadRefDeleted {
        actor: User,
    },
    Reviewed {
        state: ReviewState,
        actor: User,
        body: Option<String>,
    },
    /// The issue/PR was linked to another issue/PR for automatic closing.
    Connected {
        actor: User,
        /// The issue/PR that referenced this issue/PR.
        source: IssueOrPullRequest,
    },
    Reopened {
        actor: User,
    },
    Renamed {
        actor: User,
        from: String,
        to: String,
    },
    Locked {
        actor: User,
        reason: Option<LockReason>,
    },
    Milestoned {
        actor: User,
        title: String,
    },
    Pinned {
        actor: User,
    },
    Unpinned {
        actor: User,
    },
    /// This issue/PR was referenced by a commit
    Referenced {
        actor: User,
        commit_msg_summary: String,
        /// Whether the commit is in another repository
        cross_repository: Option<Repository>,
    },
    Mentioned,
    Subscribed,
    Unassigned {
        assignee: User,
        actor: User,
    },
    Unlocked {
        actor: User,
    },
    Unknown,
}

pub struct Comment {
    pub author: User,
    pub body: String,
}

impl From<octocrab::models::issues::Comment> for Comment {
    fn from(c: octocrab::models::issues::Comment) -> Self {
        Comment {
            author: c.user.into(),
            body: c.body.unwrap_or_default(),
        }
    }
}

pub struct Label {
    pub name: String,
    // Hex color, eg. `FBCA04`
    // pub color: String,
}

pub struct CrossReferenceSource {
    pub r#type: String,
    pub issue: IssueDeserModel,
}

pub enum ReviewState {
    Commented,
    ChangesRequested,
    Approved,
}

pub enum IssueCloser {
    Commit { abbr_oid: String },
    PullRequest { number: usize },
}

impl From<i64> for IssueCloser {
    fn from(number: i64) -> Self {
        Self::PullRequest {
            number: number as usize,
        }
    }
}

impl From<String> for IssueCloser {
    fn from(abbr_oid: String) -> Self {
        Self::Commit { abbr_oid }
    }
}

pub enum IssueOrPullRequest {
    PullRequest { number: usize, title: String },
    Issue { number: usize, title: String },
}

impl IssueOrPullRequest {
    pub fn title(&self) -> &str {
        match self {
            IssueOrPullRequest::PullRequest { title, .. } => &title,
            IssueOrPullRequest::Issue { title, .. } => &title,
        }
    }

    pub fn number(&self) -> usize {
        match *self {
            IssueOrPullRequest::PullRequest { number, .. } => number,
            IssueOrPullRequest::Issue { number, .. } => number,
        }
    }
}

pub enum LockReason {
    OffTopic,
    Resolved,
    Spam,
    TooHeated,
    Other(String),
}

pub struct Repository {
    pub name: String,
    pub owner: User,
}
