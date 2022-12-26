use crate::error::{Error, Result};
use graphql_client::{GraphQLQuery, Response};

pub async fn query<Q: GraphQLQuery>(
    vars: Q::Variables,
    octo: &octocrab::Octocrab,
) -> Result<Option<Q::ResponseData>> {
    let query = Q::build_query(vars);
    let response = octo.post("graphql", Some(&query)).await?;
    response_to_result::<Q::ResponseData>(response)
}

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

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/queries/discussion.graphql",
    response_derives = "Debug"
)]
pub struct DiscussionQuery;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/queries/discussion_search.graphql",
    response_derives = "Debug"
)]
pub struct DiscussionSearchQuery;
