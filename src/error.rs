pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("authentication with github failed")]
    Authentication,
    #[error("target html url for {api_url} not found")]
    HtmlUrlNotFound { api_url: String },
    #[error("could not communicate with github")]
    GitHub(#[from] octocrab::Error),
    #[error("rendering error")]
    CrossTerm(#[from] crossterm::ErrorKind),
}
