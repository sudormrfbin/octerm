use futures_executor::block_on;
use octocrab::{
    models::{activity::Notification, Repository},
    Octocrab, Page,
};

use crate::error::{Error, Result};

pub struct GitHub {
    octocrab: Octocrab,
    notif_cache: Option<Page<Notification>>,
}

impl GitHub {
    fn new(token: &str) -> Result<Self> {
        Ok(Self {
            octocrab: Octocrab::builder()
                .personal_token(token.to_string())
                .build()?,
            notif_cache: None,
        })
    }

    pub fn token_from_env() -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;
        Self::new(&token)
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

    /// Returns the url the notification points to.
    pub fn open_notification(&mut self, notif: &Notification) -> Result<String> {
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

    /// Get all unread notifications.
    pub fn notifications(&mut self, reload: bool) -> Result<&Page<Notification>> {
        if self.notif_cache.is_none() || reload {
            let notifs = block_on(self.octocrab.activity().notifications().list().all(true).send())
                .map_err(Error::from)?;
            self.notif_cache = Some(notifs);
        }
        return Ok(self.notif_cache.as_ref().unwrap());
    }
}
