pub mod graphql;
pub mod methods;

use std::result::Result as StdResult;

use meow::server::ServerChannel;

use octocrab::{models::activity::Notification as OctoNotification, Page};
use tokio::task::JoinHandle;

use crate::error::{Error, Result};
use crate::github::{
    DiscussionMeta, DiscussionState, Issue, IssueDeserModel, IssueMeta, Notification,
    NotificationTarget, PullRequest, PullRequestMeta, RepoMeta,
};
use crate::{ServerRequest, ServerResponse};

use self::methods::{discussion, issue_timeline, pr_timeline};

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
        .map(|n| tokio::spawn(octo_notif_to_notif(n)))
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

/// Fetch additional information about the notification from the octocrab
/// Notification model and construct a [`Notification`].
pub async fn octo_notif_to_notif(
    notif: octocrab::models::activity::Notification,
) -> Result<Notification> {
    let target = match (notif.subject.r#type.as_str(), notif.subject.url.as_ref()) {
        ("Issue", Some(url)) => {
            let issue: IssueDeserModel = octocrab::instance().get(url, None::<&()>).await?;
            NotificationTarget::Issue(IssueMeta::new(issue, RepoMeta::from(&notif.repository)))
        }
        ("PullRequest", Some(url)) => {
            let pr: octocrab::models::pulls::PullRequest =
                octocrab::instance().get(url, None::<&()>).await?;
            NotificationTarget::PullRequest(PullRequestMeta::new(
                pr,
                RepoMeta::from(&notif.repository),
            ))
        }
        ("Release", Some(url)) => {
            let release: octocrab::models::repos::Release =
                octocrab::instance().get(url, None::<&()>).await?;
            NotificationTarget::Release(release.into())
        }
        ("Discussion", _) => {
            let query_vars = graphql::discussion_search_query::Variables {
                search: format!(
                    "repo:{}/{} {}",
                    notif
                        .repository
                        .owner
                        .as_ref()
                        .map(|u| u.login.clone())
                        .unwrap_or_default(),
                    notif.repository.name,
                    notif.subject.title
                ),
            };
            let data =
                graphql::query::<graphql::DiscussionSearchQuery>(query_vars, &octocrab::instance())
                    .await?;
            let convert_to_meta = || -> Option<DiscussionMeta> {
                use graphql::discussion_search_query::DiscussionSearchQuerySearchEdgesNode as ResultType;

                data?
                    .search
                    .edges?
                    .into_iter()
                    .next()??
                    .node
                    .and_then(|res| match res {
                        ResultType::Discussion(d) => Some(DiscussionMeta {
                            repo: RepoMeta::from(&notif.repository),
                            title: notif.subject.title.clone(),
                            state: match d.answer_chosen_at {
                                Some(_) => DiscussionState::Answered,
                                None => DiscussionState::Unanswered,
                            },
                            number: d.number as usize,
                        }),
                        _ => None,
                    })
            };

            convert_to_meta()
                .map(NotificationTarget::Discussion)
                .unwrap_or(NotificationTarget::Unknown)
        }
        ("CheckSuite", _) => NotificationTarget::CiBuild,
        (_, _) => NotificationTarget::Unknown,
    };

    Ok(Notification {
        inner: notif,
        target,
    })
}

/// Helper struct used to send the parameters for a issues timeline api call.
#[derive(serde::Serialize)]
struct TimelineParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    per_page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<usize>,
}
