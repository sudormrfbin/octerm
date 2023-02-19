use std::ops::Not;
use std::sync::Arc;

use std::result::Result as StdResult;

use octocrab::Octocrab;
use octocrab::{models::activity::Notification as OctoNotification, Page};
use tokio::task::JoinHandle;

use crate::error::{Error, Result};
use crate::github::{self, events::Event};
use crate::github::{
    events, Discussion, DiscussionMeta, DiscussionReplyToSuggestedAnswer, DiscussionState,
    DiscussionSuggestedAnswer, IssueDeserModel, IssueMeta, Notification, NotificationTarget,
    PullRequestMeta, RepoMeta,
};

use super::graphql;

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

pub async fn pr_timeline(
    octo: &Octocrab,
    owner: &str,
    repo: &str,
    number: usize,
) -> Result<Option<Vec<Event>>> {
    let query_vars = graphql::pull_request_timeline_query::Variables {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
        number: number as i64,
    };

    let data = graphql::query::<graphql::PullRequestTimelineQuery>(query_vars, octo).await?;

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

    Ok(convert_to_events())
}

pub async fn issue_timeline(
    octo: &Octocrab,
    owner: &str,
    repo: &str,
    number: usize,
) -> Result<Option<Vec<Event>>> {
    let query_vars = graphql::issue_timeline_query::Variables {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
        number: number as i64,
    };

    let data = graphql::query::<graphql::IssueTimelineQuery>(query_vars, octo).await?;

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

    Ok(convert_to_events())
}

pub async fn discussion(octo: &Octocrab, meta: DiscussionMeta) -> Result<Option<Discussion>> {
    let query_vars = graphql::discussion_query::Variables {
        owner: meta.repo.owner.clone(),
        repo: meta.repo.name.clone(),
        number: meta.number as i64,
    };
    let data = graphql::query::<graphql::DiscussionQuery>(query_vars, octo).await?;
    let convert_to_discussion = move || -> Option<Discussion> {
        let disc = data?.repository?.discussion?;
        let answers = disc
            .comments
            .nodes?
            .into_iter()
            .filter_map(|ans| {
                let ans = ans?;
                Some(DiscussionSuggestedAnswer {
                    author: actor!(ans, author),
                    is_answer: ans.is_answer,
                    upvotes: ans.upvote_count as usize,
                    body: ans.body,
                    created_at: ans.created_at,
                    replies: ans
                        .replies
                        .nodes
                        .into_iter()
                        .flatten()
                        .filter_map(|reply| {
                            let reply = reply?;
                            Some(DiscussionReplyToSuggestedAnswer {
                                author: actor!(reply, author),
                                body: reply.body,
                                created_at: reply.created_at,
                            })
                        })
                        .collect(),
                })
            })
            .collect();

        Some(Discussion {
            meta,
            author: actor!(disc, author),
            upvotes: disc.upvote_count as usize,
            body: disc.body,
            created_at: disc.created_at,
            suggested_answers: answers,
        })
    };
    Ok(convert_to_discussion())
}

async fn get_all_notifs(octo: Arc<Octocrab>) -> Result<Vec<OctoNotification>> {
    let mut notifs = octo.activity().notifications().list().send().await?;
    let n_pages = match notifs.number_of_pages() {
        None | Some(0) | Some(1) => return Ok(notifs.take_items()),
        Some(p) => p,
    };

    // TODO: Use Vec::with_capacity more
    // Spawn Notification::from_octocrab(n) inside each page task (halves waiting time)
    let mut tasks: Vec<JoinHandle<Result<Page<OctoNotification>>>> =
        Vec::with_capacity(n_pages as usize - 1);
    for i in 2..=n_pages {
        let octo = Arc::clone(&octo);
        tasks.push(tokio::spawn(async move {
            Ok(octo
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

pub async fn notifications(octo: Arc<Octocrab>) -> Result<Vec<Notification>> {
    let notifs = get_all_notifs(Arc::clone(&octo)).await?;
    let tasks: Vec<JoinHandle<Result<Notification>>> = notifs
        .into_iter()
        .map(|n| tokio::spawn(octo_notif_to_notif(Arc::clone(&octo), n)))
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

    Ok(result)
}

/// Fetch additional information about the notification from the octocrab
/// Notification model and construct a [`Notification`].
pub async fn octo_notif_to_notif(
    octo: Arc<Octocrab>,
    notif: octocrab::models::activity::Notification,
) -> Result<Notification> {
    let target = match (notif.subject.r#type.as_str(), notif.subject.url.as_ref()) {
        ("Issue", Some(url)) => {
            let issue: IssueDeserModel = octo.get(url, None::<&()>).await?;
            NotificationTarget::Issue(IssueMeta::new(issue, RepoMeta::from(&notif.repository)))
        }
        ("PullRequest", Some(url)) => {
            let pr: octocrab::models::pulls::PullRequest = octo.get(url, None::<&()>).await?;
            NotificationTarget::PullRequest(PullRequestMeta::new(
                pr,
                RepoMeta::from(&notif.repository),
            ))
        }
        ("Release", Some(url)) => {
            let release: octocrab::models::repos::Release = octo.get(url, None::<&()>).await?;
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
            let data = graphql::query::<graphql::DiscussionSearchQuery>(query_vars, &octo).await?;
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
