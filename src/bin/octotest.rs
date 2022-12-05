use graphql_client::GraphQLQuery;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/bin/test.graphql",
    response_derives = "Debug",
)]
struct TestQuery;

#[tokio::main]
async fn main() -> Result<()> {
    let token = std::env::var("GITHUB_TOKEN")?;
    let builder = octocrab::Octocrab::builder().personal_token(token);
    octocrab::initialise(builder)?;

    test_graphql().await?;

    Ok(())
}

async fn test_graphql() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let number = args.get(1).map(|n| n.parse().unwrap()).unwrap_or(4874);

    let vars = test_query::Variables {
        owner: "helix-editor".to_string(),
        repo: "helix".to_string(),
        number,
    };
    let query = TestQuery::build_query(vars);

    let response: graphql_client::Response<test_query::ResponseData> =
        octocrab::instance().post("graphql", Some(&query)).await?;
    let data = response.data.unwrap();
    let edges = data.repository
        .unwrap()
        .issue
        .unwrap()
        .timeline_items
        .edges
        .unwrap();
    dbg!(edges);

    Ok(())
}
