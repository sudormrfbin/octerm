pub mod events;

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::error::Result;

use self::events::Event;

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
            NotificationTarget::Discussion => 60,
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

    /// Fetch additional information about the notification from the octocrab
    /// Notification model and construct a [`Notification`].
    pub async fn from_octocrab(notif: octocrab::models::activity::Notification) -> Result<Self> {
        let url = match notif.subject.url.as_ref() {
            Some(url) => url,
            None => {
                return Ok(Notification {
                    target: match notif.subject.r#type.as_str() {
                        "Discussion" => NotificationTarget::Discussion,
                        "CheckSuite" => NotificationTarget::CiBuild,
                        // Issues and PRs usually have a subject url,
                        // so this is somewhat an edge case.
                        _ => NotificationTarget::Unknown,
                    },
                    inner: notif,
                });
            }
        };
        let target = match notif.subject.r#type.as_str() {
            "Issue" => {
                let issue: IssueDeserModel = octocrab::instance().get(url, None::<&()>).await?;
                NotificationTarget::Issue(IssueMeta::new(issue, RepoMeta::from(&notif.repository)))
            }
            "PullRequest" => {
                let pr: octocrab::models::pulls::PullRequest =
                    octocrab::instance().get(url, None::<&()>).await?;
                NotificationTarget::PullRequest(PullRequestMeta::new(
                    pr,
                    RepoMeta::from(&notif.repository),
                ))
            }
            "Release" => {
                let release: octocrab::models::repos::Release =
                    octocrab::instance().get(url, None::<&()>).await?;
                NotificationTarget::Release(release.into())
            }
            "Discussion" => NotificationTarget::Discussion,
            "CheckSuite" => NotificationTarget::CiBuild,
            _ => NotificationTarget::Unknown,
        };
        Ok(Notification {
            inner: notif,
            target,
        })
    }
}

#[derive(Clone)]
pub enum NotificationTarget {
    Issue(IssueMeta),
    PullRequest(PullRequestMeta),
    Release(ReleaseMeta),
    Discussion,
    CiBuild,
    Unknown,
}

impl NotificationTarget {
    pub fn icon(&self) -> &'static str {
        match *self {
            NotificationTarget::Issue(ref i) => i.icon(),
            NotificationTarget::PullRequest(ref p) => p.icon(),
            NotificationTarget::Release(ref r) => r.icon(),
            NotificationTarget::Discussion => "",
            NotificationTarget::CiBuild => "",
            NotificationTarget::Unknown => "",
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
#[derive(Deserialize)]
pub struct IssueDeserModel {
    pub title: String,
    pub number: usize,
    pub body: Option<String>,
    #[serde(rename = "user")]
    pub author: User,
    pub state: String,
    pub state_reason: Option<String>,
}

#[derive(Clone)]
pub struct IssueMeta {
    pub repo: RepoMeta,
    pub title: String,
    pub body: String,
    pub number: usize,
    pub author: User,
    pub state: IssueState,
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

#[derive(Serialize, Deserialize)]
pub struct IssueComment {
    #[serde(rename = "user")]
    pub author: User,
    pub body: Option<String>,
}

impl From<octocrab::models::issues::Comment> for IssueComment {
    fn from(c: octocrab::models::issues::Comment) -> Self {
        IssueComment {
            author: c.user.into(),
            body: c.body,
        }
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
    pub unique: String,
    pub author: String,
    pub state: PullRequestState,
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
            unique: pr.number.to_string(),
            author: pr.user.clone().map(|u| u.login.clone()).unwrap_or_default(),
            state,
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

#[derive(Clone, Serialize, Deserialize)]
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

impl From<octocrab::models::User> for User {
    fn from(u: octocrab::models::User) -> Self {
        Self { name: u.login }
    }
}
