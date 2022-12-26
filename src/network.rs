pub mod graphql;

use std::ops::Not;
use std::result::Result as StdResult;

use meow::server::ServerChannel;

use octocrab::{models::activity::Notification as OctoNotification, Page};
use tokio::task::JoinHandle;

use crate::error::{Error, Result};
use crate::github::events::Event;
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

    let data =
        graphql::query::<graphql::PullRequestTimelineQuery>(query_vars, &octocrab::instance()).await?;

    let convert_to_events = move || -> Option<Vec<github::events::Event>> {
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
                TimelineEvent::AddedToProjectEvent => Event::unknown("AddedToProjectEvent"),
                TimelineEvent::AutoMergeDisabledEvent => Event::unknown("AutoMergeDisabledEvent"),
                TimelineEvent::AutoMergeEnabledEvent => Event::unknown("AutoMergeEnabledEvent"),
                TimelineEvent::AutoRebaseEnabledEvent => Event::unknown("AutoRebaseEnabledEvent"),
                TimelineEvent::AutoSquashEnabledEvent => Event::unknown("AutoSquashEnabledEvent"),
                TimelineEvent::AutomaticBaseChangeFailedEvent => {
                    Event::unknown("AutomaticBaseChangeFailedEvent")
                }
                TimelineEvent::AutomaticBaseChangeSucceededEvent => {
                    Event::unknown("AutomaticBaseChangeSucceededEvent")
                }
                TimelineEvent::BaseRefChangedEvent => Event::unknown("BaseRefChangedEvent"),
                TimelineEvent::BaseRefDeletedEvent => Event::unknown("BaseRefDeletedEvent"),
                TimelineEvent::BaseRefForcePushedEvent => Event::unknown("BaseRefForcePushedEvent"),
                TimelineEvent::CommentDeletedEvent => Event::unknown("CommentDeletedEvent"),
                TimelineEvent::ConvertedNoteToIssueEvent => {
                    Event::unknown("ConvertedNoteToIssueEvent")
                }
                TimelineEvent::ConvertedToDiscussionEvent(_) => {
                    Event::unknown("ConvertedToDiscussionEvent")
                }
                TimelineEvent::DemilestonedEvent(_) => Event::unknown("DemilestonedEvent"),
                TimelineEvent::DeployedEvent => Event::unknown("DeployedEvent"),
                TimelineEvent::DeploymentEnvironmentChangedEvent => {
                    Event::unknown("DeploymentEnvironmentChangedEvent")
                }
                TimelineEvent::DisconnectedEvent => Event::unknown("DisconnectedEvent"),
                TimelineEvent::HeadRefRestoredEvent => Event::unknown("HeadRefRestoredEvent"),
                TimelineEvent::MovedColumnsInProjectEvent => {
                    Event::unknown("MovedColumnsInProjectEvent")
                }
                TimelineEvent::PullRequestCommitCommentThread => {
                    Event::unknown("PullRequestCommitCommentThread")
                }
                TimelineEvent::PullRequestReviewThread(_) => {
                    Event::unknown("PullRequestReviewThread")
                }
                TimelineEvent::PullRequestRevisionMarker => {
                    Event::unknown("PullRequestRevisionMarker")
                }
                TimelineEvent::RemovedFromProjectEvent => Event::unknown("RemovedFromProjectEvent"),
                TimelineEvent::ReviewDismissedEvent => Event::unknown("ReviewDismissedEvent"),
                TimelineEvent::ReviewRequestRemovedEvent(_) => {
                    Event::unknown("ReviewRequestRemovedEvent")
                }
                TimelineEvent::TransferredEvent => Event::unknown("TransferredEvent"),
                TimelineEvent::UnsubscribedEvent => Event::unknown("UnsubscribedEvent"),
                TimelineEvent::UserBlockedEvent => Event::unknown("UserBlockedEvent"),

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

                    EventKind::Assigned { assignee }.with(actor!(assigned), assigned.created_at)
                }

                TimelineEvent::ClosedEvent(closed) => {
                    let closer = closed.closer.map(|c| match c {
                        Closer::Commit(c) => c.abbreviated_oid.into(),
                        Closer::PullRequest(pr) => pr.number.into(),
                    });
                    EventKind::Closed { closer }.with(actor!(closed), closed.created_at)
                }

                TimelineEvent::ConnectedEvent(connected) => EventKind::Connected {
                    source: issue_or_pr!(connected.source, ConnectedSource),
                }
                .with(actor!(connected), connected.created_at),

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

                    source: issue_or_pr!(cross.source, CrossRefSource),
                }
                .with(actor!(cross), cross.created_at),
                TimelineEvent::IssueComment(comment) => EventKind::Commented { body: comment.body }
                    .with(actor!(comment, author), comment.created_at),
                TimelineEvent::LabeledEvent(labeled) => EventKind::Labeled {
                    label: events::Label {
                        name: labeled.label.name,
                    },
                }
                .with(actor!(labeled), labeled.created_at),

                TimelineEvent::LockedEvent(locked) => {
                    let reason = locked.lock_reason.map(|l| match l {
                        LockReason::OFF_TOPIC => events::LockReason::OffTopic,
                        LockReason::RESOLVED => events::LockReason::Resolved,
                        LockReason::SPAM => events::LockReason::Spam,
                        LockReason::TOO_HEATED => events::LockReason::TooHeated,
                        LockReason::Other(s) => events::LockReason::Other(s),
                    });
                    EventKind::Locked { reason }.with(actor!(locked), locked.created_at)
                }

                TimelineEvent::MarkedAsDuplicateEvent(dup) => EventKind::MarkedAsDuplicate {
                    original: dup.canonical.map(|c| issue_or_pr!(c, DuplicateCanonical)),
                }
                .with(actor!(dup), dup.created_at),
                TimelineEvent::MilestonedEvent(milestone) => EventKind::Milestoned {
                    title: milestone.milestone_title,
                }
                .with(actor!(milestone), milestone.created_at),
                TimelineEvent::PinnedEvent(pinned) => {
                    EventKind::Pinned {}.with(actor!(pinned), pinned.created_at)
                }

                TimelineEvent::ReferencedEvent(refer) => {
                    let repo = refer.is_cross_repository.then(|| events::Repository {
                        name: refer.commit_repository.name,
                        owner: refer.commit_repository.owner.login.into(),
                    });
                    let commit_msg = refer.commit.map(|c| c.message_headline).unwrap_or_default();
                    EventKind::Referenced {
                        commit_msg_summary: commit_msg,
                        cross_repository: repo,
                    }
                    .with(actor!(refer), refer.created_at)
                }

                TimelineEvent::RenamedTitleEvent(rename) => EventKind::Renamed {
                    from: rename.previous_title,
                    to: rename.current_title,
                }
                .with(actor!(rename), rename.created_at),
                TimelineEvent::ReopenedEvent(reopen) => {
                    EventKind::Reopened {}.with(actor!(reopen), reopen.created_at)
                }

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
                    }
                    .with(actor!(unassigned), unassigned.created_at)
                }

                TimelineEvent::UnlabeledEvent(unlabeled) => EventKind::Unlabeled {
                    label: events::Label {
                        name: unlabeled.label.name,
                    },
                }
                .with(actor!(unlabeled), unlabeled.created_at),
                TimelineEvent::UnlockedEvent(unlock) => {
                    EventKind::Unlocked {}.with(actor!(unlock), unlock.created_at)
                }
                TimelineEvent::UnmarkedAsDuplicateEvent(notdup) => {
                    EventKind::UnmarkedAsDuplicate {}.with(actor!(notdup), notdup.created_at)
                }
                TimelineEvent::UnpinnedEvent(unpin) => {
                    EventKind::Unpinned {}.with(actor!(unpin), unpin.created_at)
                }
                TimelineEvent::SubscribedEvent => EventKind::Subscribed.anonymous(),
                TimelineEvent::MentionedEvent => EventKind::Mentioned.anonymous(),

                TimelineEvent::ConvertToDraftEvent(draft) => {
                    EventKind::MarkedAsDraft {}.with(actor!(draft), draft.created_at)
                }
                TimelineEvent::HeadRefDeletedEvent(refdel) => EventKind::HeadRefDeleted {
                    branch: refdel.head_ref_name,
                }
                .with(actor!(refdel), refdel.created_at),
                TimelineEvent::HeadRefForcePushedEvent(reforce) => EventKind::HeadRefForcePushed {
                    before_commit_abbr_oid: reforce
                        .before_commit
                        .map(|c| c.abbreviated_oid)
                        .unwrap_or_default(),
                    after_commit_abbr_oid: reforce
                        .after_commit
                        .map(|c| c.abbreviated_oid)
                        .unwrap_or_default(),
                }
                .with(actor!(reforce), reforce.created_at),
                TimelineEvent::MergedEvent(merged) => EventKind::Merged {
                    base_branch: merged.merge_ref_name,
                }
                .with(actor!(merged), merged.created_at),
                TimelineEvent::PullRequestCommit(committed) => {
                    let author = committed
                        .commit
                        .committer
                        .and_then(|c| c.user.map(|u| u.login).or(c.name))
                        .unwrap_or_default()
                        .into();
                    EventKind::Committed {
                        message_headline: committed.commit.message_headline,
                        abbreviated_oid: committed.commit.abbreviated_oid,
                        // TODO: Check commit author too
                    }
                    .with(author, committed.commit.committed_date)
                }
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

                    body: review.body.is_empty().not().then(|| review.body),
                }
                .with(actor!(review, author), review.created_at),
                TimelineEvent::ReadyForReviewEvent(ready) => {
                    EventKind::MarkedAsReadyForReview {}.with(actor!(ready), ready.created_at)
                }
                TimelineEvent::ReviewRequestedEvent(req) => EventKind::ReviewRequested {
                    requested_reviewer: req
                        .requested_reviewer
                        .map(|r| match r {
                            Reviewer::Mannequin(u) => u.login,
                            Reviewer::Team(u) => u.name,
                            Reviewer::User(u) => u.login,
                        })
                        .unwrap_or_default()
                        .into(),
                }
                .with(actor!(req), req.created_at),
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

    let data =
        graphql::query::<graphql::IssueTimelineQuery>(query_vars, &octocrab::instance()).await?;

    let convert_to_events = move || -> Option<Vec<github::events::Event>> {
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
                TimelineEvent::AddedToProjectEvent => Event::unknown("AddedToProjectEvent"),
                TimelineEvent::CommentDeletedEvent => Event::unknown("CommentDeletedEvent"),
                TimelineEvent::ConvertedNoteToIssueEvent => {
                    Event::unknown("ConvertedNoteToIssueEvent")
                }
                TimelineEvent::ConvertedToDiscussionEvent(_) => {
                    Event::unknown("ConvertedToDiscussionEvent")
                }
                TimelineEvent::DemilestonedEvent(_) => Event::unknown("DemilestonedEvent"),
                TimelineEvent::UnsubscribedEvent => Event::unknown("UnsubscribedEvent"),
                TimelineEvent::UserBlockedEvent => Event::unknown("UserBlockedEvent"),
                TimelineEvent::TransferredEvent => Event::unknown("TransferredEvent"),
                TimelineEvent::RemovedFromProjectEvent => Event::unknown("RemovedFromProjectEvent"),
                TimelineEvent::MovedColumnsInProjectEvent => {
                    Event::unknown("MovedColumnsInProjectEvent")
                }
                TimelineEvent::DisconnectedEvent => Event::unknown("DisconnectedEvent"),

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

                    EventKind::Assigned { assignee }.with(actor!(assigned), assigned.created_at)
                }

                TimelineEvent::ClosedEvent(closed) => {
                    let closer = closed.closer.map(|c| match c {
                        Closer::Commit(c) => c.abbreviated_oid.into(),
                        Closer::PullRequest(pr) => pr.number.into(),
                    });
                    EventKind::Closed { closer }.with(actor!(closed), closed.created_at)
                }

                TimelineEvent::ConnectedEvent(connected) => EventKind::Connected {
                    source: issue_or_pr!(connected.source, ConnectedSource),
                }
                .with(actor!(connected), connected.created_at),

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

                    source: issue_or_pr!(cross.source, CrossRefSource),
                }
                .with(actor!(cross), cross.created_at),
                TimelineEvent::IssueComment(comment) => EventKind::Commented { body: comment.body }
                    .with(actor!(comment, author), comment.created_at),
                TimelineEvent::LabeledEvent(labeled) => EventKind::Labeled {
                    label: events::Label {
                        name: labeled.label.name,
                    },
                }
                .with(actor!(labeled), labeled.created_at),

                TimelineEvent::LockedEvent(locked) => {
                    let reason = locked.lock_reason.map(|l| match l {
                        LockReason::OFF_TOPIC => events::LockReason::OffTopic,
                        LockReason::RESOLVED => events::LockReason::Resolved,
                        LockReason::SPAM => events::LockReason::Spam,
                        LockReason::TOO_HEATED => events::LockReason::TooHeated,
                        LockReason::Other(s) => events::LockReason::Other(s),
                    });
                    EventKind::Locked { reason }.with(actor!(locked), locked.created_at)
                }

                TimelineEvent::MarkedAsDuplicateEvent(dup) => EventKind::MarkedAsDuplicate {
                    original: dup.canonical.map(|c| issue_or_pr!(c, DuplicateCanonical)),
                }
                .with(actor!(dup), dup.created_at),
                TimelineEvent::MilestonedEvent(milestone) => EventKind::Milestoned {
                    title: milestone.milestone_title,
                }
                .with(actor!(milestone), milestone.created_at),
                TimelineEvent::PinnedEvent(pinned) => {
                    EventKind::Pinned {}.with(actor!(pinned), pinned.created_at)
                }

                TimelineEvent::ReferencedEvent(refer) => {
                    let repo = refer.is_cross_repository.then(|| events::Repository {
                        name: refer.commit_repository.name,
                        owner: refer.commit_repository.owner.login.into(),
                    });
                    let commit_msg = refer.commit.map(|c| c.message_headline).unwrap_or_default();
                    EventKind::Referenced {
                        commit_msg_summary: commit_msg,
                        cross_repository: repo,
                    }
                    .with(actor!(refer), refer.created_at)
                }

                TimelineEvent::RenamedTitleEvent(rename) => EventKind::Renamed {
                    from: rename.previous_title,
                    to: rename.current_title,
                }
                .with(actor!(rename), rename.created_at),
                TimelineEvent::ReopenedEvent(reopen) => {
                    EventKind::Reopened {}.with(actor!(reopen), reopen.created_at)
                }

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
                    }
                    .with(actor!(unassigned), unassigned.created_at)
                }

                TimelineEvent::UnlabeledEvent(unlabeled) => EventKind::Unlabeled {
                    label: events::Label {
                        name: unlabeled.label.name,
                    },
                }
                .with(actor!(unlabeled), unlabeled.created_at),
                TimelineEvent::UnlockedEvent(unlock) => {
                    EventKind::Unlocked {}.with(actor!(unlock), unlock.created_at)
                }
                TimelineEvent::UnmarkedAsDuplicateEvent(notdup) => {
                    EventKind::UnmarkedAsDuplicate {}.with(actor!(notdup), notdup.created_at)
                }
                TimelineEvent::UnpinnedEvent(unpin) => {
                    EventKind::Unpinned {}.with(actor!(unpin), unpin.created_at)
                }
                TimelineEvent::SubscribedEvent => EventKind::Subscribed.anonymous(),
                TimelineEvent::MentionedEvent => EventKind::Mentioned.anonymous(),
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
