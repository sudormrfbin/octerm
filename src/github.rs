pub mod events;

use std::fmt::Display;

use crossterm::style::Stylize;
use serde::{Deserialize, Serialize};

use self::events::{DateTimeUtc, Event};

#[derive(Clone)]
pub struct Notification {
    pub inner: octocrab::models::activity::Notification,
    pub target: NotificationTarget,
}

impl PartialEq for Notification {
    fn eq(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }
}

impl Notification {
    pub fn to_colored_string(&self) -> String {
        let color = crate::util::notif_target_color(&self.target).into();
        let number = self
            .target
            .number()
            .map(|n| format!("{}{}", "#".dark_grey(), n.to_string().dark_grey()))
            .unwrap_or_default();
        format!(
            "{repo}{number}: {icon} {title}",
            repo = self.inner.repository.name,
            icon = self.target.icon().with(color),
            title = self.inner.subject.title.as_str().with(color),
        )
    }

    /// A sorting function that assigns ranks to a notification based on how
    /// relavant/irrelavant it is. A higher score means it can be marked as
    /// read quicker/needs less attention than a notification with a lower score.
    /// Update time of a notification is used as a tie breaker, and older
    /// notifications show up first in each rank set.
    pub fn sorter(&self) -> impl Ord {
        let irrelavance = match self.target {
            NotificationTarget::Release(_) => 100,
            NotificationTarget::PullRequest(PullRequestMeta {
                state: PullRequestState::Merged,
                ..
            }) => 90,
            NotificationTarget::Discussion(DiscussionMeta {
                state: DiscussionState::Answered,
                ..
            }) => 85,
            NotificationTarget::PullRequest(PullRequestMeta {
                state: PullRequestState::Closed,
                ..
            }) => 80,
            NotificationTarget::Issue(IssueMeta {
                state: IssueState::Closed(IssueClosedReason::NotPlanned),
                ..
            }) => 70,
            NotificationTarget::Issue(IssueMeta {
                state: IssueState::Closed(IssueClosedReason::Completed),
                ..
            }) => 65,
            NotificationTarget::Discussion(DiscussionMeta {
                state: DiscussionState::Unanswered,
                ..
            }) => 60,
            NotificationTarget::Issue(IssueMeta {
                state: IssueState::Open,
                ..
            }) => 50,
            NotificationTarget::PullRequest(PullRequestMeta {
                state: PullRequestState::Open,
                ..
            }) => 40,
            NotificationTarget::CiBuild => 30,
            NotificationTarget::Unknown => 0,
        };

        (irrelavance, std::cmp::Reverse(self.inner.updated_at))
    }
}

#[derive(Clone)]
pub enum NotificationTarget {
    Issue(IssueMeta),
    PullRequest(PullRequestMeta),
    Release(ReleaseMeta),
    Discussion(DiscussionMeta),
    CiBuild,
    Unknown,
}

impl NotificationTarget {
    pub fn icon(&self) -> &'static str {
        match *self {
            NotificationTarget::Issue(ref i) => i.icon(),
            NotificationTarget::PullRequest(ref p) => p.icon(),
            NotificationTarget::Release(ref r) => r.icon(),
            NotificationTarget::Discussion(ref d) => d.icon(),
            NotificationTarget::CiBuild => "",
            NotificationTarget::Unknown => "",
        }
    }

    pub fn number(&self) -> Option<usize> {
        match self {
            NotificationTarget::Issue(i) => Some(i.number),
            NotificationTarget::PullRequest(p) => Some(p.number),
            NotificationTarget::Release(_) => None,
            NotificationTarget::Discussion(d) => Some(d.number),
            NotificationTarget::CiBuild => None,
            NotificationTarget::Unknown => None,
        }
    }
}

#[derive(Clone)]
pub struct RepoMeta {
    pub name: String,
    pub owner: String,
}

impl From<&octocrab::models::Repository> for RepoMeta {
    fn from(r: &octocrab::models::Repository) -> Self {
        RepoMeta {
            name: r.name.clone(),
            owner: r
                .owner
                .as_ref()
                .map(|u| u.login.clone())
                .unwrap_or_default(),
        }
    }
}

/// A struct used solely for deserializing json from calling the issue API.
#[derive(Serialize, Deserialize)]
pub struct IssueDeserModel {
    pub title: String,
    pub number: usize,
    pub body: Option<String>,
    #[serde(rename = "user")]
    pub author: User,
    pub state: String,
    pub state_reason: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Clone)]
pub struct IssueMeta {
    pub repo: RepoMeta,
    pub title: String,
    pub body: String,
    pub number: usize,
    pub author: User,
    pub state: IssueState,
    pub created_at: DateTimeUtc,
}

impl IssueMeta {
    pub fn new(issue: IssueDeserModel, repo: RepoMeta) -> Self {
        let state = match (issue.state.as_str(), issue.state_reason.as_deref()) {
            ("open", _) => IssueState::Open,
            ("closed", Some("completed")) => IssueState::Closed(IssueClosedReason::Completed),
            ("closed", Some("not_planned")) => IssueState::Closed(IssueClosedReason::NotPlanned),
            _ => IssueState::Closed(IssueClosedReason::NotPlanned),
        };
        Self {
            repo,
            title: issue.title,
            body: issue
                .body
                .unwrap_or_else(|| "No description provided.".to_string()),
            number: issue.number,
            author: issue.author,
            state,
            created_at: issue.created_at,
        }
    }
}

impl IssueMeta {
    pub fn icon(&self) -> &'static str {
        match self.state {
            IssueState::Open => "",
            IssueState::Closed(_) => "",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed(IssueClosedReason),
}

impl IssueState {
    pub fn is_open(&self) -> bool {
        matches!(self, IssueState::Open)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, IssueState::Closed(_))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum IssueClosedReason {
    // Done, closed, fixed, resolved, etc.
    Completed,
    // Won't fix, duplicate stale, etc.
    NotPlanned,
}

impl Display for IssueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Open => "Open",
                Self::Closed(_) => "Closed",
            }
        )
    }
}

pub struct Issue {
    pub meta: IssueMeta,
    pub events: Vec<Event>,
}

impl Issue {
    pub fn new(meta: IssueMeta, events: Vec<Event>) -> Self {
        Self { meta, events }
    }
}

#[derive(Clone)]
pub struct PullRequestMeta {
    pub repo: RepoMeta,
    pub title: String,
    pub body: String,
    pub number: usize,
    pub author: User,
    pub state: PullRequestState,
    pub created_at: DateTimeUtc,
}

impl PullRequestMeta {
    pub fn new(pr: octocrab::models::pulls::PullRequest, repo: RepoMeta) -> Self {
        let state = match pr.merged_at {
            Some(_) => PullRequestState::Merged,
            None => match pr.closed_at {
                Some(_) => PullRequestState::Closed,
                None => PullRequestState::Open,
            },
        };
        Self {
            repo,
            title: pr.title.clone().unwrap_or_default(),
            body: pr
                .body
                .clone()
                .unwrap_or_else(|| "No description provided.".to_string()),
            number: pr.number as usize,
            author: pr.user.map(|u| User::from(*u)).unwrap_or_default(),
            state,
            created_at: pr.created_at.unwrap_or_default(),
        }
    }
}

impl PullRequestMeta {
    pub fn icon(&self) -> &'static str {
        match self.state {
            PullRequestState::Open => "",
            PullRequestState::Merged => "",
            PullRequestState::Closed => "",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

impl PullRequestState {
    pub fn is_open(&self) -> bool {
        matches!(self, PullRequestState::Open)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, PullRequestState::Closed)
    }

    pub fn is_merged(&self) -> bool {
        matches!(self, PullRequestState::Merged)
    }
}

impl Display for PullRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Open => "Open",
                Self::Closed => "Closed",
                Self::Merged => "Merged",
            }
        )
    }
}

pub struct PullRequest {
    pub meta: PullRequestMeta,
    pub events: Vec<Event>,
}

impl PullRequest {
    pub fn new(meta: PullRequestMeta, events: Vec<Event>) -> Self {
        Self { meta, events }
    }
}

#[derive(Clone)]
pub struct ReleaseMeta {
    pub title: String,
    pub body: String,
    pub author: String,
    pub tag_name: String,
}

impl ReleaseMeta {
    pub fn icon(&self) -> &'static str {
        ""
    }
}

impl From<octocrab::models::repos::Release> for ReleaseMeta {
    fn from(release: octocrab::models::repos::Release) -> Self {
        let title = release
            .name
            .clone()
            .unwrap_or_else(|| release.tag_name.clone());
        Self {
            title,
            body: release
                .body
                .clone()
                .unwrap_or_else(|| "No description provided.".to_string()),
            author: release.author.login,
            tag_name: release.tag_name,
        }
    }
}

#[derive(Clone)]
pub struct DiscussionMeta {
    pub repo: RepoMeta,
    pub title: String,
    pub number: usize,
    pub state: DiscussionState,
}

impl DiscussionMeta {
    pub fn icon(&self) -> &'static str {
        ""
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DiscussionState {
    Answered,
    Unanswered,
}

pub struct Discussion {
    pub meta: DiscussionMeta,
    pub author: User,
    pub upvotes: usize,
    pub body: String,
    pub created_at: DateTimeUtc,
    pub suggested_answers: Vec<DiscussionSuggestedAnswer>,
}

pub struct DiscussionSuggestedAnswer {
    pub author: User,
    pub is_answer: bool,
    pub upvotes: usize,
    pub body: String,
    pub created_at: DateTimeUtc,
    pub replies: Vec<DiscussionReplyToSuggestedAnswer>,
}

pub struct DiscussionReplyToSuggestedAnswer {
    pub author: User,
    pub body: String,
    pub created_at: DateTimeUtc,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct User {
    /// The username with which the user logs in; the @ name.
    #[serde(rename = "login")]
    pub name: String,
}

impl User {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("@")?;
        f.write_str(&self.name)
    }
}

impl From<octocrab::models::User> for User {
    fn from(u: octocrab::models::User) -> Self {
        Self { name: u.login }
    }
}

impl From<String> for User {
    fn from(name: String) -> Self {
        Self { name }
    }
}
