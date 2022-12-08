pub mod graphql;

use std::ops::Not;
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
            $gql_type::PullRequest(pr) => $crate::github::events::IssueOrPullRequest::PullRequest {
                number: pr.number as usize,
                title: pr.title,
            },
        }
    };
}

async fn open_pr(pr: PullRequestMeta, send: impl Fn(ServerResponse)) -> Result<()> {
    let query_vars = graphql::pull_request_timeline_query::Variables {
        owner: pr.repo.owner.clone(),
        repo: pr.repo.name.clone(),
        number: pr.number as i64,
    };

    let query = graphql::PullRequestTimelineQuery::build_query(query_vars);
    let response = octocrab::instance().post("graphql", Some(&query)).await?;
    let data = graphql::response_to_result::<
        <graphql::PullRequestTimelineQuery as GraphQLQuery>::ResponseData,
    >(response)?;

    let convert_to_events = move || -> Option<Vec<github::events::EventKind>> {
        use github::events::EventKind;
        use graphql::pull_request_timeline_query::*;
        use PullRequestTimelineQueryRepositoryPullRequestTimelineItemsEdgesNode as TimelineEvent;
        use PullRequestTimelineQueryRepositoryPullRequestTimelineItemsEdgesNodeOnAssignedEventAssignee as Assignee;
        use PullRequestTimelineQueryRepositoryPullRequestTimelineItemsEdgesNodeOnClosedEventCloser as Closer;
        use PullRequestTimelineQueryRepositoryPullRequestTimelineItemsEdgesNodeOnConnectedEventSource as ConnectedSource;
        use PullRequestTimelineQueryRepositoryPullRequestTimelineItemsEdgesNodeOnCrossReferencedEventSource as CrossRefSource;
        use PullRequestTimelineQueryRepositoryPullRequestTimelineItemsEdgesNodeOnMarkedAsDuplicateEventCanonical as DuplicateCanonical;
        use PullRequestTimelineQueryRepositoryPullRequestTimelineItemsEdgesNodeOnReviewRequestedEventRequestedReviewer as Reviewer;
        use PullRequestTimelineQueryRepositoryPullRequestTimelineItemsEdgesNodeOnUnassignedEventAssignee as Unassignee;

        let events = data?
            .repository?
            .pull_request?
            .timeline_items
            .edges?
            .into_iter()
            .filter_map(|e| e?.node)
            .map(|node| match node {
                TimelineEvent::AddedToProjectEvent => EventKind::Unknown("AddedToProjectEvent"),
                TimelineEvent::AutoMergeDisabledEvent => EventKind::Unknown("AutoMergeDisabledEvent"),
                TimelineEvent::AutoMergeEnabledEvent => EventKind::Unknown("AutoMergeEnabledEvent"),
                TimelineEvent::AutoRebaseEnabledEvent => EventKind::Unknown("AutoRebaseEnabledEvent"),
                TimelineEvent::AutoSquashEnabledEvent => EventKind::Unknown("AutoSquashEnabledEvent"),
                TimelineEvent::AutomaticBaseChangeFailedEvent => {
                    EventKind::Unknown("AutomaticBaseChangeFailedEvent")
                }
                TimelineEvent::AutomaticBaseChangeSucceededEvent => {
                    EventKind::Unknown("AutomaticBaseChangeSucceededEvent")
                }
                TimelineEvent::BaseRefChangedEvent => EventKind::Unknown("BaseRefChangedEvent"),
                TimelineEvent::BaseRefDeletedEvent => EventKind::Unknown("BaseRefDeletedEvent"),
                TimelineEvent::BaseRefForcePushedEvent => EventKind::Unknown("BaseRefForcePushedEvent"),
                TimelineEvent::CommentDeletedEvent => EventKind::Unknown("CommentDeletedEvent"),
                TimelineEvent::ConvertedNoteToIssueEvent => {
                    EventKind::Unknown("ConvertedNoteToIssueEvent")
                }
                TimelineEvent::ConvertedToDiscussionEvent(_) => {
                    EventKind::Unknown("ConvertedToDiscussionEvent")
                }
                TimelineEvent::DemilestonedEvent(_) => EventKind::Unknown("DemilestonedEvent"),
                TimelineEvent::DeployedEvent => EventKind::Unknown("DeployedEvent"),
                TimelineEvent::DeploymentEnvironmentChangedEvent => {
                    EventKind::Unknown("DeploymentEnvironmentChangedEvent")
                }
                TimelineEvent::DisconnectedEvent => EventKind::Unknown("DisconnectedEvent"),
                TimelineEvent::HeadRefRestoredEvent => EventKind::Unknown("HeadRefRestoredEvent"),
                TimelineEvent::MovedColumnsInProjectEvent => {
                    EventKind::Unknown("MovedColumnsInProjectEvent")
                }
                TimelineEvent::PullRequestCommitCommentThread => {
                    EventKind::Unknown("PullRequestCommitCommentThread")
                }
                TimelineEvent::PullRequestReviewThread(_) => {
                    EventKind::Unknown("PullRequestReviewThread")
                }
                TimelineEvent::PullRequestRevisionMarker => {
                    EventKind::Unknown("PullRequestRevisionMarker")
                }
                TimelineEvent::RemovedFromProjectEvent => EventKind::Unknown("RemovedFromProjectEvent"),
                TimelineEvent::ReviewDismissedEvent => EventKind::Unknown("ReviewDismissedEvent"),
                TimelineEvent::ReviewRequestRemovedEvent(_) => {
                    EventKind::Unknown("ReviewRequestRemovedEvent")
                }
                TimelineEvent::TransferredEvent => EventKind::Unknown("TransferredEvent"),
                TimelineEvent::UnsubscribedEvent => EventKind::Unknown("UnsubscribedEvent"),
                TimelineEvent::UserBlockedEvent => EventKind::Unknown("UserBlockedEvent"),

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

                    EventKind::Assigned {
                        assignee,
                        actor: actor!(assigned),
                    }
                }

                TimelineEvent::ClosedEvent(closed) => {
                    let closer = closed.closer.map(|c| match c {
                        Closer::Commit(c) => c.abbreviated_oid.into(),
                        Closer::PullRequest(pr) => pr.number.into(),
                    });
                    EventKind::Closed {
                        actor: actor!(closed),
                        closer,
                    }
                }

                TimelineEvent::ConnectedEvent(connected) => EventKind::Connected {
                    actor: actor!(connected),
                    source: issue_or_pr!(connected.source, ConnectedSource),
                },

                TimelineEvent::CrossReferencedEvent(cross) => EventKind::CrossReferenced {
                    cross_repository: cross.is_cross_repository.then(|| match cross.source {
                        CrossRefSource::Issue(ref i) => events::Repository {
                            name: i.repository.name.clone(),
                            owner: i.repository.owner.login.clone().into(),
                        },
                        CrossRefSource::PullRequest(ref pr) => events::Repository {
                            name: pr.repository.name.clone(),
                            owner: pr.repository.owner.login.clone().into(),
                        },
                    }),
                    actor: actor!(cross),
                    source: issue_or_pr!(cross.source, CrossRefSource),
                },
                TimelineEvent::IssueComment(comment) => EventKind::Commented(events::Comment {
                    author: actor!(comment, author),
                    body: comment.body,
                }),
                TimelineEvent::LabeledEvent(labeled) => EventKind::Labeled {
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
                    EventKind::Locked {
                        actor: actor!(locked),
                        reason,
                    }
                }

                TimelineEvent::MarkedAsDuplicateEvent(dup) => EventKind::MarkedAsDuplicate {
                    actor: actor!(dup),
                    original: dup.canonical.map(|c| issue_or_pr!(c, DuplicateCanonical)),
                },
                TimelineEvent::MilestonedEvent(milestone) => EventKind::Milestoned {
                    actor: actor!(milestone),
                    title: milestone.milestone_title,
                },
                TimelineEvent::PinnedEvent(pinned) => EventKind::Pinned {
                    actor: actor!(pinned),
                },

                TimelineEvent::ReferencedEvent(refer) => {
                    let repo = refer.is_cross_repository.then(|| events::Repository {
                        name: refer.commit_repository.name,
                        owner: refer.commit_repository.owner.login.into(),
                    });
                    let commit_msg = refer.commit.map(|c| c.message_headline).unwrap_or_default();
                    EventKind::Referenced {
                        actor: actor!(refer),
                        commit_msg_summary: commit_msg,
                        cross_repository: repo,
                    }
                }

                TimelineEvent::RenamedTitleEvent(rename) => EventKind::Renamed {
                    actor: actor!(rename),
                    from: rename.previous_title,
                    to: rename.current_title,
                },
                TimelineEvent::ReopenedEvent(reopen) => EventKind::Reopened {
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
                    EventKind::Unassigned {
                        assignee: unassignee,
                        actor: actor!(unassigned),
                    }
                }

                TimelineEvent::UnlabeledEvent(unlabeled) => EventKind::Unlabeled {
                    actor: actor!(unlabeled),
                    label: events::Label {
                        name: unlabeled.label.name,
                    },
                },
                TimelineEvent::UnlockedEvent(unlock) => EventKind::Unlocked {
                    actor: actor!(unlock),
                },
                TimelineEvent::UnmarkedAsDuplicateEvent(notdup) => EventKind::UnmarkedAsDuplicate {
                    actor: actor!(notdup),
                },
                TimelineEvent::UnpinnedEvent(unpin) => EventKind::Unpinned {
                    actor: actor!(unpin),
                },
                TimelineEvent::SubscribedEvent => EventKind::Subscribed,
                TimelineEvent::MentionedEvent => EventKind::Mentioned,

                TimelineEvent::ConvertToDraftEvent(draft) => EventKind::MarkedAsDraft {
                    actor: actor!(draft),
                },
                TimelineEvent::HeadRefDeletedEvent(refdel) => EventKind::HeadRefDeleted {
                    actor: actor!(refdel),
                    branch: refdel.head_ref_name,
                },
                TimelineEvent::HeadRefForcePushedEvent(reforce) => EventKind::HeadRefForcePushed {
                    actor: actor!(reforce),
                    before_commit_abbr_oid: reforce
                        .before_commit
                        .map(|c| c.abbreviated_oid)
                        .unwrap_or_default(),
                    after_commit_abbr_oid: reforce
                        .after_commit
                        .map(|c| c.abbreviated_oid)
                        .unwrap_or_default(),
                },
                TimelineEvent::MergedEvent(merged) => EventKind::Merged {
                    actor: actor!(merged),
                    base_branch: merged.merge_ref_name,
                },
                TimelineEvent::PullRequestCommit(committed) => EventKind::Committed {
                    message_headline: committed.commit.message_headline,
                    abbreviated_oid: committed.commit.abbreviated_oid,
                    // TODO: Check commit author too
                    author: committed
                        .commit
                        .committer
                        .and_then(|c| c.user.map(|u| u.login).or(c.name))
                        .unwrap_or_default()
                        .into(),
                },
                TimelineEvent::PullRequestReview(review) => EventKind::Reviewed {
                    state: match review.state {
                        PullRequestReviewState::APPROVED => events::ReviewState::Approved,
                        PullRequestReviewState::CHANGES_REQUESTED => {
                            events::ReviewState::ChangesRequested
                        }
                        PullRequestReviewState::COMMENTED => events::ReviewState::Commented,
                        PullRequestReviewState::DISMISSED => events::ReviewState::Dismissed,
                        PullRequestReviewState::PENDING => events::ReviewState::Pending,
                        PullRequestReviewState::Other(s) => events::ReviewState::Other(s),
                    },
                    actor: actor!(review, author),
                    body: review.body.is_empty().not().then(|| review.body),
                },
                TimelineEvent::ReadyForReviewEvent(ready) => EventKind::MarkedAsReadyForReview {
                    actor: actor!(ready),
                },
                TimelineEvent::ReviewRequestedEvent(req) => EventKind::ReviewRequested {
                    actor: actor!(req),
                    requested_reviewer: req
                        .requested_reviewer
                        .map(|r| match r {
                            Reviewer::Mannequin(u) => u.login,
                            Reviewer::Team(u) => u.name,
                            Reviewer::User(u) => u.login,
                        })
                        .unwrap_or_default()
                        .into(),
                },
            })
            .collect();

        Some(events)
    };

    let events = convert_to_events().unwrap_or_default();
    send(ServerResponse::PullRequest(PullRequest::new(pr, events)));

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

    let convert_to_events = move || -> Option<Vec<github::events::EventKind>> {
        use github::events::EventKind;
        use graphql::issue_timeline_query::*;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNode as TimelineEvent;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnAssignedEventAssignee as Assignee;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnClosedEventCloser as Closer;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnConnectedEventSource as ConnectedSource;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnCrossReferencedEventSource as CrossRefSource;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnMarkedAsDuplicateEventCanonical as DuplicateCanonical;
        use IssueTimelineQueryRepositoryIssueTimelineItemsEdgesNodeOnUnassignedEventAssignee as Unassignee;

        let events = data?
            .repository?
            .issue?
            .timeline_items
            .edges?
            .into_iter()
            .filter_map(|e| e?.node)
            .map(|node| match node {
                TimelineEvent::AddedToProjectEvent => EventKind::Unknown("AddedToProjectEvent"),
                TimelineEvent::CommentDeletedEvent => EventKind::Unknown("CommentDeletedEvent"),
                TimelineEvent::ConvertedNoteToIssueEvent => {
                    EventKind::Unknown("ConvertedNoteToIssueEvent")
                }
                TimelineEvent::ConvertedToDiscussionEvent(_) => {
                    EventKind::Unknown("ConvertedToDiscussionEvent")
                }
                TimelineEvent::DemilestonedEvent(_) => EventKind::Unknown("DemilestonedEvent"),
                TimelineEvent::UnsubscribedEvent => EventKind::Unknown("UnsubscribedEvent"),
                TimelineEvent::UserBlockedEvent => EventKind::Unknown("UserBlockedEvent"),
                TimelineEvent::TransferredEvent => EventKind::Unknown("TransferredEvent"),
                TimelineEvent::RemovedFromProjectEvent => EventKind::Unknown("RemovedFromProjectEvent"),
                TimelineEvent::MovedColumnsInProjectEvent => {
                    EventKind::Unknown("MovedColumnsInProjectEvent")
                }
                TimelineEvent::DisconnectedEvent => EventKind::Unknown("DisconnectedEvent"),

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

                    EventKind::Assigned {
                        assignee,
                        actor: actor!(assigned),
                    }
                }

                TimelineEvent::ClosedEvent(closed) => {
                    let closer = closed.closer.map(|c| match c {
                        Closer::Commit(c) => c.abbreviated_oid.into(),
                        Closer::PullRequest(pr) => pr.number.into(),
                    });
                    EventKind::Closed {
                        actor: actor!(closed),
                        closer,
                    }
                }

                TimelineEvent::ConnectedEvent(connected) => EventKind::Connected {
                    actor: actor!(connected),
                    source: issue_or_pr!(connected.source, ConnectedSource),
                },

                TimelineEvent::CrossReferencedEvent(cross) => EventKind::CrossReferenced {
                    cross_repository: cross.is_cross_repository.then(|| match cross.source {
                        CrossRefSource::Issue(ref i) => events::Repository {
                            name: i.repository.name.clone(),
                            owner: i.repository.owner.login.clone().into(),
                        },
                        CrossRefSource::PullRequest(ref pr) => events::Repository {
                            name: pr.repository.name.clone(),
                            owner: pr.repository.owner.login.clone().into(),
                        },
                    }),
                    actor: actor!(cross),
                    source: issue_or_pr!(cross.source, CrossRefSource),
                },
                TimelineEvent::IssueComment(comment) => EventKind::Commented(events::Comment {
                    author: actor!(comment, author),
                    body: comment.body,
                }),
                TimelineEvent::LabeledEvent(labeled) => EventKind::Labeled {
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
                    EventKind::Locked {
                        actor: actor!(locked),
                        reason,
                    }
                }

                TimelineEvent::MarkedAsDuplicateEvent(dup) => EventKind::MarkedAsDuplicate {
                    actor: actor!(dup),
                    original: dup.canonical.map(|c| issue_or_pr!(c, DuplicateCanonical)),
                },
                TimelineEvent::MilestonedEvent(milestone) => EventKind::Milestoned {
                    actor: actor!(milestone),
                    title: milestone.milestone_title,
                },
                TimelineEvent::PinnedEvent(pinned) => EventKind::Pinned {
                    actor: actor!(pinned),
                },

                TimelineEvent::ReferencedEvent(refer) => {
                    let repo = refer.is_cross_repository.then(|| events::Repository {
                        name: refer.commit_repository.name,
                        owner: refer.commit_repository.owner.login.into(),
                    });
                    let commit_msg = refer.commit.map(|c| c.message_headline).unwrap_or_default();
                    EventKind::Referenced {
                        actor: actor!(refer),
                        commit_msg_summary: commit_msg,
                        cross_repository: repo,
                    }
                }

                TimelineEvent::RenamedTitleEvent(rename) => EventKind::Renamed {
                    actor: actor!(rename),
                    from: rename.previous_title,
                    to: rename.current_title,
                },
                TimelineEvent::ReopenedEvent(reopen) => EventKind::Reopened {
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
                    EventKind::Unassigned {
                        assignee: unassignee,
                        actor: actor!(unassigned),
                    }
                }

                TimelineEvent::UnlabeledEvent(unlabeled) => EventKind::Unlabeled {
                    actor: actor!(unlabeled),
                    label: events::Label {
                        name: unlabeled.label.name,
                    },
                },
                TimelineEvent::UnlockedEvent(unlock) => EventKind::Unlocked {
                    actor: actor!(unlock),
                },
                TimelineEvent::UnmarkedAsDuplicateEvent(notdup) => EventKind::UnmarkedAsDuplicate {
                    actor: actor!(notdup),
                },
                TimelineEvent::UnpinnedEvent(unpin) => EventKind::Unpinned {
                    actor: actor!(unpin),
                },
                TimelineEvent::SubscribedEvent => EventKind::Subscribed,
                TimelineEvent::MentionedEvent => EventKind::Mentioned,
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
