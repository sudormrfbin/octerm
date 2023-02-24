pub mod graphql;
pub mod methods;

/// Helper struct used to send the parameters for a issues timeline api call.
#[derive(serde::Serialize)]
struct TimelineParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    per_page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<usize>,
}
