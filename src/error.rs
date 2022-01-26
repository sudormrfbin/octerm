pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // TODO: Add InvalidToken and wrap Auth like RateLimit
    #[error("authentication with github failed")]
    Authentication,
    #[error("target html url for {api_url} not found")]
    HtmlUrlNotFound { api_url: String },
    #[error("could not communicate with github")]
    GitHub(#[source] octocrab::Error),
    #[error("github api rate limit exceeded")]
    GitHubRateLimitExceeded(#[source] octocrab::Error),
    #[error("rendering error")]
    CrossTerm(#[from] crossterm::ErrorKind),
    #[error("could not complete concurrent network requests")]
    NetworkTask,
}

impl From<octocrab::Error> for Error {
    fn from(e: octocrab::Error) -> Self {
        if let octocrab::Error::GitHub { ref source, .. } = e {
            if source.message.contains("rate limit exceeded") {
                return Self::GitHubRateLimitExceeded(e);
            }
        }
        return Self::GitHub(e);
    }
}
