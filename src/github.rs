use octocrab::{
    models::{activity::Notification, Repository},
    Page,
};

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
    pub cache: Option<Page<Notification>>,
}

impl NotificationStore {
    pub fn new() -> Self {
        Self { cache: None }
    }

    /// Get the nth notification in the cache.
    pub fn nth(&self, idx: usize) -> Option<&Notification> {
        self.cache.as_ref()?.items.get(idx)
    }

    /// Number of notifications in the cache.
    pub fn len(&self) -> usize {
        self.cache.as_ref().map(|c| c.items.len()).unwrap_or(0)
    }

    /// Get all unread notifications. Results are retrieved from a cache if
    /// possible. Call [`Self::refresh()`] to refresh the cache.
    pub fn unread(&mut self) -> Option<&[Notification]> {
        return self.cache.as_ref().map(|c| c.items.as_slice());
    }
}
