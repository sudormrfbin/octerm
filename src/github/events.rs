use serde::{Serialize, Deserialize};

use super::User;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "event")]
pub enum Event {
    Commented(Comment),
    #[serde(other)]
    Unknown
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
