use std::result::Result as StdResult;

use meow::server::ServerChannel;

use octocrab::{models::activity::Notification as OctoNotification, Page};
use tokio::task::JoinHandle;

use crate::error::{Error, Result};
use crate::github::{self, Issue, IssueMeta, Notification, PullRequest, PullRequestMeta};
use crate::{ServerRequest, ServerResponse};

type Channel = ServerChannel<ServerRequest, ServerResponse>;

#[tokio::main]
pub async fn start_server(channel: Channel) {
    let send = |resp| channel.send_to_app(resp).expect("ServerChannel closed");
    while let Ok(req) = channel.recv_from_app() {
        send(ServerResponse::AsyncTaskStart);

        let res = match req {
            ServerRequest::RefreshNotifs => refresh(send).await,
            ServerRequest::OpenNotifInBrowser(n) => open_in_browser(n).await,
            ServerRequest::MarkNotifAsRead(n) => mark_as_read(send, n).await,
            ServerRequest::OpenIssue(issue) => open_issue(issue, send).await,
            ServerRequest::OpenPullRequest(pr) => open_pr(pr, send).await,
        };
        send(ServerResponse::AsyncTaskDone);

        if let Err(err) = res {
            send(ServerResponse::Error(err));
        }
    }
}

async fn open_pr(pr: PullRequestMeta, send: impl Fn(ServerResponse)) -> Result<()> {
    let events = issue_timeline(pr.repo.owner.clone(), pr.repo.name.clone(), pr.number).await?;
    send(ServerResponse::PullRequest(PullRequest::new(pr, events)));

    Ok(())
}

async fn open_issue(issue: IssueMeta, send: impl Fn(ServerResponse)) -> Result<()> {
    let events = issue_timeline(
        issue.repo.owner.clone(),
        issue.repo.name.clone(),
        issue.number,
    )
    .await?;
    send(ServerResponse::Issue(Issue::new(issue, events)));

    Ok(())
}

async fn get_all_notifs() -> Result<Vec<OctoNotification>> {
    let mut notifs = octocrab::instance()
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

pub async fn refresh(send: impl Fn(ServerResponse)) -> Result<()> {
    let notifs = get_all_notifs().await?;
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

    send(ServerResponse::Notifications(result));
    Ok(())
}

pub async fn open_in_browser(notif: Notification) -> Result<()> {
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
                octocrab::instance().get(default_url?, None::<&()>).await?;
            release.html_url.to_string()
        }
        "Issue" => match notif.inner.subject.latest_comment_url {
            Some(ref url) => {
                let comment: octocrab::models::issues::Comment =
                    octocrab::instance().get(url, None::<&()>).await?;
                comment.html_url.to_string()
            }
            None => {
                // TODO: Return last (newest) comment in thread
                let issue: octocrab::models::issues::Issue =
                    octocrab::instance().get(default_url?, None::<&()>).await?;
                issue.html_url.to_string()
            }
        },
        "PullRequest" => {
            // BUG: In case of PRs, the url is simple, without the latest comment,
            // changed files, etc. Therefore the behavior is different from clicking
            // a PR notification in the web ui, which would show the latest change.
            let pr: octocrab::models::pulls::PullRequest =
                octocrab::instance().get(default_url?, None::<&()>).await?;
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
    open::that(url.as_str()).map_err(|_| Error::BrowserNotAvailable)?;

    Ok(())
}

pub async fn mark_as_read(send: impl Fn(ServerResponse), notif: Notification) -> Result<()> {
    octocrab::instance()
        .activity()
        .notifications()
        .mark_as_read(notif.inner.id)
        .await?;
    send(ServerResponse::MarkedNotifAsRead(notif));

    Ok(())
}

/// Retrieve timeline events from an issue. Github considers every pull request
/// to be an issue, so this function works with PRs too.
async fn issue_timeline(
    owner: String,
    repo: String,
    number: usize,
) -> Result<Vec<github::events::Event>> {
    let events = octocrab::instance()
        .get::<Page<github::events::Event>, String, TimelineParams>(
            format!("repos/{owner}/{repo}/issues/{number}/timeline",),
            Some(&TimelineParams {
                per_page: Some(100),
                page: None,
            }),
        )
        .await?
        .take_items();
    Ok(events)
}

/// Helper struct used to send the parameters for a issues timeline api call.
#[derive(serde::Serialize)]
struct TimelineParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    per_page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<usize>,
}
