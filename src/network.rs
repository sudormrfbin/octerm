use std::result::Result as StdResult;
use std::sync::Arc;

use tokio::sync::Mutex;

use octocrab::{models::activity::Notification as OctoNotification, Octocrab, Page};
use tokio::task::JoinHandle;

use crate::app::App;
use crate::error::{Error, Result};
use crate::events::NotifEvent;
use crate::github::Notification;

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

    async fn get_all_notifs(&self) -> Result<Vec<OctoNotification>> {
        let mut notifs = self
            .octocrab
            .activity()
            .notifications()
            .list()
            .send()
            .await?;
        let pages = match notifs.number_of_pages().filter(|p| *p > 1) {
            None => return Ok(notifs.take_items()),
            Some(p) => p,
        };

        // TODO: Use Vec::with_capacity more
        // Spawn Notification::from_octocrab(n) inside each page task (halves waiting time)
        let mut tasks: Vec<JoinHandle<Result<Page<OctoNotification>>>> =
            Vec::with_capacity(pages as usize - 1);
        for i in 2..=pages {
            tasks.push(tokio::spawn(async move {
                Ok(octocrab::instance()
                    .activity()
                    .notifications()
                    .list()
                    .page(i as u8)
                    .send()
                    .await?)
            }));
        }

        let result: Vec<StdResult<Result<Page<OctoNotification>>, tokio::task::JoinError>> =
            futures::future::join_all(tasks).await;

        let mut acc = notifs.take_items();
        acc.reserve_exact(50 * result.len()); // Max notifications from each request is 50

        let result = result.into_iter().try_fold(acc, |mut acc, task| {
            let notif = task.map_err(|_| Error::NetworkTask)?;
            acc.extend_from_slice(&notif?.take_items());
            Ok::<Vec<OctoNotification>, Error>(acc)
        })?;
        Ok(result)
    }

    pub async fn refresh(&self) -> Result<()> {
        let notifs = self.get_all_notifs().await?;
        let tasks: Vec<JoinHandle<Result<Notification>>> = notifs
            .into_iter()
            .map(|n| tokio::spawn(Notification::from_octocrab(n)))
            .collect();

        // TODO: Buffer the requests
        let result: Vec<StdResult<Result<Notification>, tokio::task::JoinError>> =
            futures::future::join_all(tasks).await;
        let vec = Vec::with_capacity(result.len());
        let mut result = result.into_iter().try_fold(vec, |mut acc, task| {
            let notif = task.map_err(|_| Error::NetworkTask)?;
            acc.push(notif?);
            Ok::<Vec<Notification>, Error>(acc)
        })?;
        result.sort_unstable_by_key(Notification::sorter);
        result.reverse();

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
        let url = match notif.inner.subject.r#type.as_str() {
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
