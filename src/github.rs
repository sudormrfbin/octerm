use octocrab::models::Repository;

#[derive(Clone)]
pub struct Notification {
    pub inner: octocrab::models::activity::Notification,
    pub target: NotificationTarget,
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

#[derive(Clone)]
pub struct Issue {
    pub inner: octocrab::models::issues::Issue,
    pub state: IssueState,
}

impl From<octocrab::models::issues::Issue> for Issue {
    fn from(issue: octocrab::models::issues::Issue) -> Self {
        let state = match issue.closed_at {
            Some(_) => IssueState::Closed,
            None => IssueState::Open,
        };
        Self {
            inner: issue,
            state,
        }
    }
}

#[derive(Clone)]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Clone)]
pub struct PullRequest {
    pub inner: octocrab::models::pulls::PullRequest,
    pub state: PullRequestState,
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
        Self { inner: pr, state }
    }
}

#[derive(Clone)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

#[derive(Clone)]
pub struct Release {
    pub inner: octocrab::models::repos::Release,
}

impl From<octocrab::models::repos::Release> for Release {
    fn from(release: octocrab::models::repos::Release) -> Self {
        Self { inner: release }
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
