pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Authentication,
    UrlNotFound,
    Octocrab(octocrab::Error),
    CrossTerm(crossterm::ErrorKind),
}

impl From<crossterm::ErrorKind> for Error {
    fn from(e: crossterm::ErrorKind) -> Self {
        Self::CrossTerm(e)
    }
}

impl From<octocrab::Error> for Error {
    fn from(e: octocrab::Error) -> Self {
        Self::Octocrab(e)
    }
}


