use meow::server::ServerChannel;

use crate::error::{Error, Result};
use crate::github::{DiscussionMeta, Issue, IssueMeta, Notification, PullRequest, PullRequestMeta};
use crate::app::{ServerRequest, ServerResponse};

use super::methods::{discussion, issue_timeline, notifications, pr_timeline, mark_notification_as_read};

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
            ServerRequest::OpenDiscussion(disc) => open_discussion(disc, send).await,
        };
        send(ServerResponse::AsyncTaskDone);

        if let Err(err) = res {
            send(ServerResponse::Error(err));
        }
    }
}

async fn open_pr(pr: PullRequestMeta, send: impl Fn(ServerResponse)) -> Result<()> {
    let events = pr_timeline(
        &octocrab::instance(),
        &pr.repo.owner,
        &pr.repo.name,
        pr.number,
    )
    .await?
    .unwrap_or_default();
    send(ServerResponse::PullRequest(PullRequest::new(pr, events)));

    Ok(())
}

async fn open_issue(issue: IssueMeta, send: impl Fn(ServerResponse)) -> Result<()> {
    let events = issue_timeline(
        &octocrab::instance(),
        &issue.repo.owner,
        &issue.repo.name,
        issue.number,
    )
    .await?
    .unwrap_or_default();
    send(ServerResponse::Issue(Issue::new(issue, events)));

    Ok(())
}

async fn open_discussion(meta: DiscussionMeta, send: impl Fn(ServerResponse)) -> Result<()> {
    if let Some(disc) = discussion(&octocrab::instance(), meta).await? {
        send(ServerResponse::Discussion(disc))
    }
    Ok(())
}

pub async fn refresh(send: impl Fn(ServerResponse)) -> Result<()> {
    send(ServerResponse::Notifications(
        notifications(octocrab::instance()).await?,
    ));
    Ok(())
}

pub async fn mark_as_read(send: impl Fn(ServerResponse), notif: Notification) -> Result<()> {
    mark_notification_as_read(&octocrab::instance(), notif.inner.id).await?;
    
    send(ServerResponse::MarkedNotifAsRead(notif));

    Ok(())
}
