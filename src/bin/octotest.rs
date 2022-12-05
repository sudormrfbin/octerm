use graphql_client::GraphQLQuery;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "src/bin/test.graphql",
    response_derives = "Debug"
)]
struct TestQuery;

#[tokio::main]
async fn main() -> Result<()> {
    let token = std::env::var("GITHUB_TOKEN")?;
    let builder = octocrab::Octocrab::builder().personal_token(token);
    octocrab::initialise(builder)?;

    // test_graphql_client().await?;
    test_cynic().await?;

    Ok(())
}

async fn test_graphql_client() -> Result<()> {
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
    let edges = response
        .data
        .unwrap()
        .repository
        .unwrap()
        .issue
        .unwrap()
        .timeline_items
        .edges
        .unwrap();
    dbg!(edges);

    Ok(())
}

async fn test_cynic() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let number = args.get(1).map(|n| n.parse().unwrap()).unwrap_or(4874);

    let vars = queries::IssueTimelineQueryVariables {
        owner: "helix-editor".to_string(),
        repo: "helix".to_string(),
        number,
    };

    use cynic::QueryBuilder;
    let query = queries::IssueTimelineQuery::build(vars);
    let response: cynic::GraphQlResponse<queries::IssueTimelineQuery> =
        octocrab::instance().post("graphql", Some(&query)).await?;
    let edges = response
        .data
        .unwrap()
        .repository
        .unwrap()
        .issue
        .unwrap()
        .timeline_items
        .edges
        .unwrap();
    dbg!(edges);

    let convert_to_events = move || -> Option<Vec<octerm::github::events::Event>> {
        use octerm::github::events::Event;

        let events = response
            .data?
            .repository?
            .issue?
            .timeline_items
            .edges?
            .into_iter()
            .filter_map(|e| match e?.node? {
                queries::IssueTimelineItems::AssignedEvent(assigned) => {
                    let assignee = assigned
                        .assignee
                        .and_then(|a| match a {
                            queries::Assignee::User(u) => Some(u.login),
                            queries::Assignee::Organization(o) => Some(o.login),
                            queries::Assignee::Mannequin(m) => Some(m.login),
                            queries::Assignee::Bot(b) => Some(b.login),
                            queries::Assignee::Unknown => None,
                        })
                        .unwrap_or_default()
                        .into();
                    let actor = assigned.actor.map(|a| a.login).unwrap_or_default().into();
                    Some(Event::Assigned { assignee, actor })
                }
                queries::IssueTimelineItems::ConnectedEvent(_) => todo!(),
                queries::IssueTimelineItems::CrossReferencedEvent(_) => todo!(),
                queries::IssueTimelineItems::IssueComment(_) => todo!(),
                queries::IssueTimelineItems::LabeledEvent(_) => todo!(),
                queries::IssueTimelineItems::ClosedEvent(_) => todo!(),
                queries::IssueTimelineItems::ConvertedToDiscussionEvent(_) => todo!(),
                queries::IssueTimelineItems::DemilestonedEvent(_) => todo!(),
                queries::IssueTimelineItems::LockedEvent(_) => todo!(),
                queries::IssueTimelineItems::MarkedAsDuplicateEvent(_) => todo!(),
                queries::IssueTimelineItems::MilestonedEvent(_) => todo!(),
                queries::IssueTimelineItems::PinnedEvent(_) => todo!(),
                queries::IssueTimelineItems::ReferencedEvent(_) => todo!(),
                queries::IssueTimelineItems::RenamedTitleEvent(_) => todo!(),
                queries::IssueTimelineItems::ReopenedEvent(_) => todo!(),
                queries::IssueTimelineItems::UnassignedEvent(_) => todo!(),
                queries::IssueTimelineItems::UnlabeledEvent(_) => todo!(),
                queries::IssueTimelineItems::UnlockedEvent(_) => todo!(),
                queries::IssueTimelineItems::UnmarkedAsDuplicateEvent(_) => todo!(),
                queries::IssueTimelineItems::UnpinnedEvent(_) => todo!(),
                queries::IssueTimelineItems::Unknown => todo!(),
            });

        Some(events)
    };

    Ok(())
}

#[cynic::schema_for_derives(file = r#"schema.graphql"#, module = "schema")]
mod queries {
    use super::schema;

    #[derive(cynic::QueryVariables, Debug)]
    pub struct IssueTimelineQueryVariables {
        pub number: i32,
        pub owner: String,
        pub repo: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct User {
        pub login: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct UnpinnedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct UnmarkedAsDuplicateEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct UnlockedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct UnlabeledEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub label: Label,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct UnassignedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub assignee: Option<Assignee>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct ReopenedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct RenamedTitleEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub current_title: String,
        pub previous_title: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct ReferencedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub is_cross_repository: bool,
        pub commit: Option<Commit>,
        pub commit_repository: Repository,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Repository {
        pub name: String,
        pub owner: RepositoryOwner,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct RepositoryOwner {
        pub login: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Query", variables = "IssueTimelineQueryVariables")]
    pub struct IssueTimelineQuery {
        #[arguments(name: $repo, owner: $owner)]
        pub repository: Option<Repository2>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Repository", variables = "IssueTimelineQueryVariables")]
    pub struct Repository2 {
        #[arguments(number: $number)]
        pub issue: Option<RepoIssue>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct PullRequest {
        pub title: String,
        pub number: i32,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct PinnedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Organization {
        pub login: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct MilestonedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub milestone_title: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct MarkedAsDuplicateEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub canonical: Option<IssueOrPullRequest>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Mannequin {
        pub login: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct LockedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub lock_reason: Option<LockReason>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct LabeledEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub label: Label,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Label {
        pub name: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct IssueComment {
        pub created_at: DateTime,
        pub author: Option<Actor>,
        pub body: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Issue")]
    pub struct Issue {
        pub title: String,
        pub number: i32,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Issue")]
    pub struct RepoIssue {
        #[arguments(first: 100)]
        pub timeline_items: IssueTimelineItemsConnection,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct IssueTimelineItemsConnection {
        pub edges: Option<Vec<Option<IssueTimelineItemsEdge>>>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct IssueTimelineItemsEdge {
        pub node: Option<IssueTimelineItems>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct DemilestonedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub milestone_title: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct CrossReferencedEvent {
        pub created_at: DateTime,
        pub source: ReferencedSubject,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct ConvertedToDiscussionEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub discussion: Option<Discussion>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Discussion {
        pub number: i32,
        pub title: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct ConnectedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub source: ReferencedSubject2,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Commit {
        pub abbreviated_oid: String,
        pub message_headline: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct ClosedEvent {
        pub created_at: DateTime,
        pub actor: Option<Actor>,
        pub closer: Option<Closer>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Bot {
        pub login: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct AssignedEvent {
        pub assignee: Option<Assignee>,
        pub created_at: DateTime,
        pub actor: Option<Actor>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct Actor {
        pub login: String,
    }

    #[derive(cynic::InlineFragments, Debug)]
    pub enum Assignee {
        User(User),
        Organization(Organization),
        Mannequin(Mannequin),
        Bot(Bot),
        #[cynic(fallback)]
        Unknown,
    }

    #[derive(cynic::InlineFragments, Debug)]
    pub enum Closer {
        PullRequest(PullRequest),
        Commit(Commit),
        #[cynic(fallback)]
        Unknown,
    }

    #[derive(cynic::InlineFragments, Debug)]
    pub enum IssueOrPullRequest {
        Issue(Issue),
        PullRequest(PullRequest),
        #[cynic(fallback)]
        Unknown,
    }

    #[derive(cynic::InlineFragments, Debug)]
    pub enum IssueTimelineItems {
        AssignedEvent(AssignedEvent),
        ConnectedEvent(ConnectedEvent),
        CrossReferencedEvent(CrossReferencedEvent),
        IssueComment(IssueComment),
        LabeledEvent(LabeledEvent),
        ClosedEvent(ClosedEvent),
        ConvertedToDiscussionEvent(ConvertedToDiscussionEvent),
        DemilestonedEvent(DemilestonedEvent),
        LockedEvent(LockedEvent),
        MarkedAsDuplicateEvent(MarkedAsDuplicateEvent),
        MilestonedEvent(MilestonedEvent),
        PinnedEvent(PinnedEvent),
        ReferencedEvent(ReferencedEvent),
        RenamedTitleEvent(RenamedTitleEvent),
        ReopenedEvent(ReopenedEvent),
        UnassignedEvent(UnassignedEvent),
        UnlabeledEvent(UnlabeledEvent),
        UnlockedEvent(UnlockedEvent),
        UnmarkedAsDuplicateEvent(UnmarkedAsDuplicateEvent),
        UnpinnedEvent(UnpinnedEvent),
        #[cynic(fallback)]
        Unknown,
    }

    #[derive(cynic::InlineFragments, Debug)]
    #[cynic(graphql_type = "ReferencedSubject")]
    pub enum ReferencedSubject2 {
        PullRequest(PullRequest),
        #[cynic(fallback)]
        Unknown,
    }

    #[derive(cynic::InlineFragments, Debug)]
    pub enum ReferencedSubject {
        PullRequest(PullRequest),
        Issue2(Issue),
        #[cynic(fallback)]
        Unknown,
    }

    #[derive(cynic::Enum, Clone, Copy, Debug)]
    pub enum LockReason {
        OffTopic,
        Resolved,
        Spam,
        TooHeated,
    }

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct DateTime(pub String);
}

#[allow(non_snake_case, non_camel_case_types)]
mod schema {
    cynic::use_schema!(r#"schema.graphql"#);
}
