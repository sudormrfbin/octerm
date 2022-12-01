use crate::github;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "event")]
pub enum Event {
    Commented(github::IssueComment),
    #[serde(other)]
    Unknown
}
