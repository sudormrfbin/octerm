pub mod graphql;

use std::result::Result as StdResult;

use graphql_client::GraphQLQuery;
use meow::server::ServerChannel;

use octocrab::{models::activity::Notification as OctoNotification, Page};
use tokio::task::JoinHandle;

use crate::error::{Error, Result};
use crate::github::{self, events, Issue, IssueMeta, Notification, PullRequest, PullRequestMeta};
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
    send(ServerResponse::PullRequest(PullRequest::new(
        pr,
        Vec::new(),
    )));

    Ok(())
}

async fn open_issue(issue: IssueMeta, send: impl Fn(ServerResponse)) -> Result<()> {
    let query_vars = graphql::issue_timeline_query::Variables {
        owner: issue.repo.owner.clone(),
        repo: issue.repo.name.clone(),
        number: issue.number as i64,
    };

    let query = graphql::IssueTimelineQuery::build_query(query_vars);
    let response = octocrab::instance().post("graphql", Some(&query)).await?;
    let data = graphql::response_to_result::<
        <graphql::IssueTimelineQuery as GraphQLQuery>::ResponseData,
    >(response)?;

    let convert_to_events = move || -> Option<Vec<github::events::Event>> {
        use github::events::Event;
        use graphql::issue_timeline_query::*;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNode as TimelineEvent;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnAssignedEventAssignee as Assignee;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnClosedEventCloser as Closer;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnConnectedEventSource as ConnectedSource;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnCrossReferencedEventSource as CrossRefSource;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnMarkedAsDuplicateEventCanonical as DuplicateCanonical;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnUnassignedEventAssignee as Unassignee;

        macro_rules! actor {
            ($root:expr) => {
                actor!($root, actor)
            };
            ($root:expr, $actor_token:ident) => {
                $crate::github::User::from($root.$actor_token.map(|a| a.login).unwrap_or_default())
            };
        }

        macro_rules! issue_or_pr {
            ($var:expr, $gql_type:ident) => {
                match $var {
                    $gql_type::Issue(i) => $crate::github::events::IssueOrPullRequest::Issue {
                        number: i.number as usize,
                        title: i.title,
                    },
                    $gql_type::PullRequest(pr) => {
                        $crate::github::events::IssueOrPullRequest::PullRequest {
                            number: pr.number as usize,
                            title: pr.title,
                        }
                    }
                }
            };
        }

        let events = data?
            .repository?
            .issue?
            .timeline_items
            .edges?
            .into_iter()
            .filter_map(|e| e?.node)
            .map(|node| match node {
                TimelineEvent::AddedToProjectEvent => Event::Unknown,
                TimelineEvent::CommentDeletedEvent => Event::Unknown,
                TimelineEvent::ConvertedNoteToIssueEvent => Event::Unknown,
                TimelineEvent::ConvertedToDiscussionEvent(_) => Event::Unknown,
                TimelineEvent::DemilestonedEvent(_) => Event::Unknown,
                TimelineEvent::UnsubscribedEvent => Event::Unknown,
                TimelineEvent::UserBlockedEvent => Event::Unknown,
                TimelineEvent::TransferredEvent => Event::Unknown,
                TimelineEvent::RemovedFromProjectEvent => Event::Unknown,
                TimelineEvent::MovedColumnsInProjectEvent => Event::Unknown,
                TimelineEvent::DisconnectedEvent => Event::Unknown,

                TimelineEvent::AssignedEvent(assigned) => {
                    let assignee = assigned
                        .assignee
                        .map(|a| match a {
                            Assignee::Bot(b) => b.login,
                            Assignee::Mannequin(m) => m.login,
                            Assignee::Organization(o) => o.login,
                            Assignee::User(u) => u.login,
                        })
                        .unwrap_or_default()
                        .into();

                    Event::Assigned {
                        assignee,
                        actor: actor!(assigned),
                    }
                }

                TimelineEvent::ClosedEvent(closed) => {
                    let closer = closed.closer.map(|c| match c {
                        Closer::Commit(c) => c.abbreviated_oid.into(),
                        Closer::PullRequest(pr) => pr.number.into(),
                    });
                    Event::Closed {
                        actor: actor!(closed),
                        closer,
                    }
                }

                TimelineEvent::ConnectedEvent(connected) => Event::Connected {
                    actor: actor!(connected),
                    source: issue_or_pr!(connected.source, ConnectedSource),
                },

                TimelineEvent::CrossReferencedEvent(cross) => Event::CrossReferenced {
                    actor: actor!(cross),
                    source: issue_or_pr!(cross.source, CrossRefSource),
                },
                TimelineEvent::IssueComment(comment) => Event::Commented(events::Comment {
                    author: actor!(comment, author),
                    body: comment.body,
                }),
                TimelineEvent::LabeledEvent(labeled) => Event::Labeled {
                    actor: actor!(labeled),
                    label: events::Label {
                        name: labeled.label.name,
                    },
                },

                TimelineEvent::LockedEvent(locked) => {
                    let reason = locked.lock_reason.map(|l| match l {
                        LockReason::OFF_TOPIC => events::LockReason::OffTopic,
                        LockReason::RESOLVED => events::LockReason::Resolved,
                        LockReason::SPAM => events::LockReason::Spam,
                        LockReason::TOO_HEATED => events::LockReason::TooHeated,
                        LockReason::Other(s) => events::LockReason::Other(s),
                    });
                    Event::Locked {
                        actor: actor!(locked),
                        reason,
                    }
                }

                TimelineEvent::MarkedAsDuplicateEvent(dup) => Event::MarkedAsDuplicate {
                    actor: actor!(dup),
                    original: dup.canonical.map(|c| issue_or_pr!(c, DuplicateCanonical)),
                },
                TimelineEvent::MilestonedEvent(milestone) => Event::Milestoned {
                    actor: actor!(milestone),
                    title: milestone.milestone_title,
                },
                TimelineEvent::PinnedEvent(pinned) => Event::Pinned {
                    actor: actor!(pinned),
                },

                TimelineEvent::ReferencedEvent(refer) => {
                    let repo = refer.is_cross_repository.then(|| events::Repository {
                        name: refer.commit_repository.name,
                        owner: refer.commit_repository.owner.login.into(),
                    });
                    let commit_msg = refer.commit.map(|c| c.message_headline).unwrap_or_default();
                    Event::Referenced {
                        actor: actor!(refer),
                        commit_msg_summary: commit_msg,
                        cross_repository: repo,
                    }
                }

                TimelineEvent::RenamedTitleEvent(rename) => Event::Renamed {
                    actor: actor!(rename),
                    from: rename.previous_title,
                    to: rename.current_title,
                },
                TimelineEvent::ReopenedEvent(reopen) => Event::Reopened {
                    actor: actor!(reopen),
                },

                TimelineEvent::UnassignedEvent(unassigned) => {
                    let unassignee = unassigned
                        .assignee
                        .map(|a| match a {
                            Unassignee::Bot(b) => b.login,
                            Unassignee::Mannequin(m) => m.login,
                            Unassignee::Organization(o) => o.login,
                            Unassignee::User(u) => u.login,
                        })
                        .unwrap_or_default()
                        .into();
                    Event::Unassigned {
                        assignee: unassignee,
                        actor: actor!(unassigned),
                    }
                }

                TimelineEvent::UnlabeledEvent(unlabeled) => Event::Unlabeled {
                    actor: actor!(unlabeled),
                    label: events::Label {
                        name: unlabeled.label.name,
                    },
                },
                TimelineEvent::UnlockedEvent(unlock) => Event::Unlocked {
                    actor: actor!(unlock),
                },
                TimelineEvent::UnmarkedAsDuplicateEvent(notdup) => Event::UnmarkedAsDuplicate {
                    actor: actor!(notdup),
                },
                TimelineEvent::UnpinnedEvent(unpin) => Event::Unpinned {
                    actor: actor!(unpin),
                },
                TimelineEvent::SubscribedEvent => Event::Subscribed,
                TimelineEvent::MentionedEvent => Event::Mentioned,
            })
            .collect();

        Some(events)
    };

    let events = convert_to_events().unwrap_or_default();
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

/// Helper struct used to send the parameters for a issues timeline api call.
#[derive(serde::Serialize)]
struct TimelineParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    per_page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<usize>,
}
