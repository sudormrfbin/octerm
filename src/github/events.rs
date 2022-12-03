use serde::{Deserialize, Serialize};

use super::{IssueDeserModel, User};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "event")]
pub enum Event {
    Commented(Comment),
    Merged {
        actor: User,
        /// The commit that was merged into the main branch
        commit_id: String,
    },
    Closed {
        actor: User,
    },
    Committed {
        message: String,
    },
    Labeled {
        actor: User,
        label: Label,
    },
    Unlabeled {
        actor: User,
        label: Label,
    },
    // Only this event is kebab-cased in the response, probably a slip-up
    #[serde(rename = "cross-referenced")]
    CrossReferenced {
        actor: User,
        source: CrossReferenceSource,
    },
    HeadRefForcePushed {
        actor: User,
    },
    HeadRefDeleted {
        actor: User,
    },
    Reviewed {
        state: ReviewState,
        #[serde(rename = "user")]
        actor: User, // rename for consistency with other events
        body: Option<String>,
    },
    Connected {
        actor: User,
    },
    Reopened {
        actor: User,
    },
    Mentioned,
    Subscribed,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Deserialize)]
pub struct Comment {
    #[serde(rename = "user")]
    pub author: User,
    pub body: Option<String>,
}

impl From<octocrab::models::issues::Comment> for Comment {
    fn from(c: octocrab::models::issues::Comment) -> Self {
        Comment {
            author: c.user.into(),
            body: c.body,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    /// Hex color, eg. `FBCA04`
    pub color: String,
}

#[derive(Serialize, Deserialize)]
pub struct CrossReferenceSource {
    pub r#type: String,
    pub issue: IssueDeserModel,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    Commented,
    ChangesRequested,
    Approved,
}
