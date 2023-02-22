use super::User;

pub type DateTimeLocal = chrono::DateTime<chrono::Local>;
pub type DateTimeUtc = chrono::DateTime<chrono::Utc>;

pub struct Event {
    pub actor: User,
    pub created_at: DateTimeLocal,
    pub kind: EventKind,
}

impl Event {
    pub fn unknown(ev: &'static str) -> Self {
        Event {
            actor: User { name: "".into() },
            created_at: DateTimeLocal::default(),
            kind: EventKind::Unknown(ev),
        }
    }
}

pub enum EventKind {
    Assigned {
        assignee: User,
    },
    Commented {
        body: String,
    },
    Merged {
        /// The branch into which the PR was merged (main,master, etc)
        base_branch: String,
    },
    Closed {
        /// The issue was closed automatically because a PR/commit was linked
        /// here and was merged/committed.
        closer: Option<IssueCloser>,
    },
    Committed {
        message_headline: String,
        abbreviated_oid: String,
    },
    Labeled {
        label: Label,
    },
    Unlabeled {
        label: Label,
    },
    MarkedAsDuplicate {
        original: Option<IssueOrPullRequest>,
    },
    UnmarkedAsDuplicate,
    CrossReferenced {
        source: IssueOrPullRequest,
        /// Whether the referring issue/PR is in another repository
        cross_repository: Option<Repository>,
    },
    HeadRefForcePushed {
        before_commit_abbr_oid: String,
        after_commit_abbr_oid: String,
    },
    HeadRefDeleted {
        /// Deleted branch
        branch: String,
    },
    MarkedAsDraft,
    MarkedAsReadyForReview,
    ReviewRequested {
        requested_reviewer: User,
    },
    Reviewed {
        state: ReviewState,
        body: Option<String>,
    },
    /// The issue/PR was linked to another issue/PR for automatic closing.
    Connected {
        /// The issue/PR that referenced this issue/PR.
        source: IssueOrPullRequest,
    },
    Reopened,
    Renamed {
        from: String,
        to: String,
    },
    Locked {
        reason: Option<LockReason>,
    },
    Milestoned {
        title: String,
    },
    Pinned,
    Unpinned,
    /// This issue/PR was referenced by a commit
    Referenced {
        commit_msg_summary: String,
        /// Whether the commit is in another repository
        cross_repository: Option<Repository>,
    },
    Mentioned,
    Subscribed,
    Unassigned {
        assignee: User,
    },
    Unlocked,
    /// Unhandled event, with name of the event
    Unknown(&'static str),
}

impl EventKind {
    /// Create an Event from an EventKind with the supplied actor and date.
    /// Useful as builder pattern. UTC time is converted to local time.
    pub fn with(self, actor: User, created_at: DateTimeUtc) -> Event {
        Event {
            actor,
            created_at: created_at.into(),
            kind: self,
        }
    }

    /// Create an event with no author and date. The event does have these
    /// attributes, but we simply do not care about them for some events like
    /// Subscribed, Mentioned, etc
    pub fn anonymous(self) -> Event {
        Event {
            kind: self,
            created_at: DateTimeLocal::default(),
            actor: User::new(""),
        }
    }
}

pub struct Label {
    pub name: String,
    // Hex color, eg. `FBCA04`
    // pub color: String,
}

pub enum ReviewState {
    Commented,
    ChangesRequested,
    Approved,
    Dismissed,
    Pending,
    Other(String),
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
            IssueOrPullRequest::PullRequest { title, .. } => title,
            IssueOrPullRequest::Issue { title, .. } => title,
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
