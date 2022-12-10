use crate::error::{Error, Result};
use graphql_client::Response;

pub fn response_to_result<Data>(resp: Response<Data>) -> Result<Option<Data>> {
    if let Some(err) = resp.errors {
        return Err(Error::Graphql(err));
    }
    Ok(resp.data)
}

pub type DateTime = crate::github::events::DateTimeUtc;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/queries/issue.graphql",
    response_derives = "Debug"
)]
pub struct IssueTimelineQuery;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/queries/pr.graphql",
    response_derives = "Debug"
)]
pub struct PullRequestTimelineQuery;
