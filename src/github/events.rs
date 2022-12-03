use serde::{Deserialize, Serialize};

use super::{User, IssueDeserModel};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "event")]
pub enum Event {
    Commented(Comment),
    Merged {
        actor: User,
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
    CrossReferenced {
        actor: User,
        source: CrossReferenceSource,
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
