use std::result::Result as StdResult;
use std::sync::Arc;

use tokio::sync::Mutex;

use octocrab::Octocrab;
use tokio::task::JoinHandle;

use crate::app::App;
use crate::error::{Error, Result};
use crate::events::NotifEvent;
use crate::github::{Notification, NotificationTarget};

pub struct Network {
    octocrab: Octocrab,
    pub app: Arc<Mutex<App>>,
}

impl Network {
    pub fn new(octocrab: Octocrab, app: Arc<Mutex<App>>) -> Self {
        Self { octocrab, app }
    }

    pub async fn handle_event(&mut self, event: NotifEvent) -> Result<()> {
        match event {
            NotifEvent::Refresh => self.refresh().await,
            NotifEvent::Open(notif) => self.open(&notif).await,
            NotifEvent::MarkAsRead(notif) => self.mark_as_read(&notif).await,
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        let notifs = self
            .octocrab
            .activity()
            .notifications()
            .list()
            .send()
            .await?;

        let mut tasks: Vec<JoinHandle<Result<Notification>>> = Vec::new();
        for notif in notifs.into_iter() {
            tasks.push(tokio::spawn(async move {
                let url = match notif.subject.url.as_ref() {
                    Some(url) => url,
                    None => {
                        return Ok(Notification {
                            target: match notif.subject.type_.as_str() {
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
                let target = match notif.subject.type_.as_str() {
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
            }));
        }
        // TODO: Buffer the requests
        let result: Vec<StdResult<Result<Notification>, tokio::task::JoinError>> =
            futures::future::join_all(tasks).await;
        let vec = Vec::with_capacity(result.len());
        let mut result =
            result
                .into_iter()
                .try_fold(vec, |mut acc, task| {
                    let notif = task.map_err(|_| Error::NetworkTask)?;
                    acc.push(notif?);
                    Ok::<Vec<Notification>, Error>(acc)
                })?;
        result.sort_unstable_by_key(|n| n.inner.updated_at);
        result.reverse(); // Most recent on top

        let mut app = self.app.lock().await;
        app.github.notif.cache = Some(result);
        Ok(())
    }

    pub async fn open(&mut self, notif: &Notification) -> Result<()> {
        let default_url = notif
            .inner
            .subject
            .url
            .as_ref()
            .ok_or(Error::HtmlUrlNotFound {
                api_url: notif.inner.url.to_string(),
            });
        let url = match notif.inner.subject.type_.as_str() {
            "Release" => {
                let release: octocrab::models::repos::Release =
                    self.octocrab.get(default_url?, None::<&()>).await?;
                release.html_url.to_string()
            }
            "Issue" => match notif.inner.subject.latest_comment_url {
                Some(ref url) => {
                    let comment: octocrab::models::issues::Comment =
                        self.octocrab.get(url, None::<&()>).await?;
                    comment.html_url.to_string()
                }
                None => {
                    // TODO: Return last (newest) comment in thread
                    let issue: octocrab::models::issues::Issue =
                        self.octocrab.get(default_url?, None::<&()>).await?;
                    issue.html_url.to_string()
                }
            },
            "PullRequest" => {
                // BUG: In case of PRs, the url is simple, without the latest comment,
                // changed files, etc. Therefore the behavior is different from clicking
                // a PR notification in the web ui, which would show the latest change.
                let pr: octocrab::models::pulls::PullRequest =
                    self.octocrab.get(default_url?, None::<&()>).await?;
                pr.html_url
                    .ok_or(Error::HtmlUrlNotFound {
                        api_url: notif.inner.url.to_string(),
                    })?
                    .to_string()
            }
            _ => {
                return Err(Error::HtmlUrlNotFound {
                    api_url: notif.inner.url.to_string(),
                })
            }
        };

        let mut app = self.app.lock().await;
        app.state.open_url = Some(url);
        Ok(())
    }

    pub async fn mark_as_read(&mut self, notif: &Notification) -> Result<()> {
        self.octocrab
            .activity()
            .notifications()
            .mark_as_read(notif.inner.id)
            .await?;

        let mut app = self.app.lock().await;
        if let Some(ref mut v) = app.github.notif.cache {
            let idx = v.iter().position(|n| n.inner.id == notif.inner.id).unwrap();
            v.remove(idx);
        }
        Ok(())
    }
}
