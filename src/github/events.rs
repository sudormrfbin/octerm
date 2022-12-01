use crate::github;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    Commented(github::IssueComment),
}
