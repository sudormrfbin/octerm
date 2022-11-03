use std::fmt::Display;

use crate::error::Result;
use octocrab::models::Repository;

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
            NotificationTarget::PullRequest(PullRequest {
                state: PullRequestState::Merged,
                ..
            }) => 90,
            NotificationTarget::PullRequest(PullRequest {
                state: PullRequestState::Closed,
                ..
            }) => 80,
            NotificationTarget::Issue(Issue {
                state: IssueState::Closed,
                ..
            }) => 70,
            NotificationTarget::Discussion => 60,
            NotificationTarget::Issue(Issue {
                state: IssueState::Open,
                ..
            }) => 50,
            NotificationTarget::PullRequest(PullRequest {
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
                let issue: octocrab::models::issues::Issue =
                    octocrab::instance().get(url, None::<&()>).await?;
                NotificationTarget::Issue(issue.into())
            }
            "PullRequest" => {
                let pr: octocrab::models::pulls::PullRequest =
                    octocrab::instance().get(url, None::<&()>).await?;
                NotificationTarget::PullRequest(pr.into())
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
    Issue(Issue),
    PullRequest(PullRequest),
    Release(Release),
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
pub struct Issue {
    pub inner: octocrab::models::issues::Issue,
    pub title: String,
    pub body: String,
    pub unique: String,
    pub author: String,
    pub state: IssueState,
}

impl Issue {
    pub fn icon(&self) -> &'static str {
        match self.state {
            IssueState::Open => "",
            IssueState::Closed => "",
        }
    }
}

impl From<octocrab::models::issues::Issue> for Issue {
    fn from(issue: octocrab::models::issues::Issue) -> Self {
        let state = match issue.closed_at {
            Some(_) => IssueState::Closed,
            None => IssueState::Open,
        };
        Self {
            title: issue.title.clone(),
            body: issue
                .body
                .clone()
                .unwrap_or("No description provided.".to_string()),
            unique: issue.number.to_string(),
            author: issue.user.login.clone(),
            inner: issue,
            state,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
}

impl Display for IssueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Open => "Open",
                Self::Closed => "Closed",
            }
        )
    }
}

#[derive(Clone)]
pub struct PullRequest {
    pub inner: octocrab::models::pulls::PullRequest,
    pub title: String,
    pub body: String,
    pub unique: String,
    pub author: String,
    pub state: PullRequestState,
}

impl PullRequest {
    pub fn icon(&self) -> &'static str {
        match self.state {
            PullRequestState::Open => "",
            PullRequestState::Merged => "",
            PullRequestState::Closed => "",
        }
    }
}

impl From<octocrab::models::pulls::PullRequest> for PullRequest {
    fn from(pr: octocrab::models::pulls::PullRequest) -> Self {
        let state = match pr.merged_at {
            Some(_) => PullRequestState::Merged,
            None => match pr.closed_at {
                Some(_) => PullRequestState::Closed,
                None => PullRequestState::Open,
            },
        };
        Self {
            title: pr.title.clone().unwrap_or_default(),
            body: pr
                .body
                .clone()
                .unwrap_or("No description provided.".to_string()),
            unique: pr.number.to_string(),
            author: pr.user.clone().map(|u| u.login.clone()).unwrap_or_default(),
            state,
            inner: pr,
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
pub struct Release {
    pub title: String,
    pub body: String,
    pub unique: String,
    pub inner: octocrab::models::repos::Release,
}

impl Release {
    pub fn icon(&self) -> &'static str {
        return "";
    }
}

impl From<octocrab::models::repos::Release> for Release {
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
                .unwrap_or("No description provided.".to_string()),
            unique: release.tag_name.clone(),
            inner: release,
        }
    }
}

pub struct GitHub {
    pub notif: NotificationStore,
}

impl GitHub {
    pub fn new() -> Self {
        Self {
            notif: NotificationStore::new(),
        }
    }

    /// Constructs a "repo_author/repo_name" string normally seen on GitHub.
    pub fn repo_name(repo: &Repository) -> String {
        let name = repo.name.as_str();
        let author = repo
            .owner
            .as_ref()
            .map(|o| o.login.clone())
            .unwrap_or_default();
        format!("{author}/{name}")
    }
}

pub struct NotificationStore {
    pub cache: Option<Vec<Notification>>,
}

impl NotificationStore {
    pub fn new() -> Self {
        Self { cache: None }
    }

    /// Get the nth notification in the cache.
    pub fn nth(&self, idx: usize) -> Option<&Notification> {
        self.cache.as_ref()?.get(idx)
    }

    /// Number of notifications in the cache.
    pub fn len(&self) -> usize {
        self.cache.as_ref().map(|v| v.len()).unwrap_or(0)
    }

    /// Get all unread notifications. Results are retrieved from a cache if
    /// possible. Call [`Self::refresh()`] to refresh the cache.
    pub fn unread(&mut self) -> Option<&[Notification]> {
        return self.cache.as_ref().map(|v| v.as_slice());
    }
}
