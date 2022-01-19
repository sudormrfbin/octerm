use futures_executor::block_on;
use octocrab::{
    models::{activity::Notification, Repository},
    Octocrab, Page,
};

use crate::error::{Error, Result};

pub struct GitHub<'gh> {
    pub octocrab: &'gh Octocrab,
    pub notif: NotificationStore<'gh>,
}

impl<'gh> GitHub<'gh> {
    pub fn new(octocrab_: &'gh Octocrab) -> Result<Self> {
        Ok(Self {
            notif: NotificationStore::new(octocrab_),
            octocrab: octocrab_,
        })
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

pub struct NotificationStore<'octo> {
    octocrab: &'octo Octocrab,
    cache: Option<Page<Notification>>,
}

impl<'octo> NotificationStore<'octo> {
    pub fn new(octo: &'octo Octocrab) -> Self {
        Self {
            octocrab: octo,
            cache: None,
        }
    }

    /// Get the nth notification in the cache.
    pub fn nth(&self, idx: usize) -> Option<&Notification> {
        self.cache.as_ref()?.items.iter().nth(idx)
    }

    /// Number of notifications in the cache.
    pub fn len(&self) -> usize {
        self.cache.as_ref().map(|c| c.items.len()).unwrap_or(0)
    }

    /// Get all unread notifications. Results are retrieved from a cache if
    /// possible. Call [`Self::refresh()`] to refresh the cache.
    pub fn get_unread(&mut self) -> Result<&[Notification]> {
        if self.cache.is_none() {
            self.refresh()?;
        }
        return Ok(self.cache.as_ref().unwrap().items.as_slice());
    }

    pub fn refresh(&mut self) -> Result<()> {
        let notifs = block_on(self.octocrab.activity().notifications().list().send())
            .map_err(Error::from)?;
        self.cache = Some(notifs);
        Ok(())
    }

    /// Returns the url a notification points to.
    pub fn open(&mut self, notif: &Notification) -> Result<String> {
        let default_url = notif.subject.url.as_ref().ok_or(Error::UrlNotFound);
        match notif.subject.type_.as_str() {
            "Release" => {
                let release: octocrab::models::repos::Release =
                    block_on(octocrab::instance().get(default_url?, None::<&()>))?;
                return Ok(release.html_url.to_string());
            }
            "Issue" => match notif.subject.latest_comment_url {
                Some(ref url) => {
                    let comment: octocrab::models::issues::Comment =
                        block_on(octocrab::instance().get(url, None::<&()>))?;
                    return Ok(comment.html_url.to_string());
                }
                None => {
                    // TODO: Return last (newest) comment in thread
                    let issue: octocrab::models::issues::Issue =
                        block_on(octocrab::instance().get(default_url?, None::<&()>))?;
                    return Ok(issue.html_url.to_string());
                }
            },
            "PullRequest" => {
                // BUG: In case of PRs, the url is simple, without the latest comment,
                // changed files, etc. Therefore the behavior is different from clicking
                // a PR notification in the web ui, which would show the latest change.
                let pr: octocrab::models::pulls::PullRequest =
                    block_on(octocrab::instance().get(default_url?, None::<&()>))?;
                return Ok(pr.html_url.ok_or(Error::UrlNotFound)?.to_string());
            }
            _ => return Err(Error::UrlNotFound),
        }
    }

    pub fn mark_as_read(&mut self, notif: &Notification) -> Result<()> {
        block_on(
            self.octocrab
                .activity()
                .notifications()
                .mark_as_read(notif.id),
        )?;
        if let Some(ref mut c) = self.cache {
            let idx = c.items.iter().position(|n| n.id == notif.id).unwrap();
            c.items.remove(idx);
        }
        Ok(())
    }
}
