pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    AuthError,
    OctocrabError(octocrab::Error),
    CrossTermError(crossterm::ErrorKind),
}

impl From<crossterm::ErrorKind> for Error {
    fn from(e: crossterm::ErrorKind) -> Self {
        Self::CrossTermError(e)
    }
}

impl From<octocrab::Error> for Error {
    fn from(e: octocrab::Error) -> Self {
        Self::OctocrabError(e)
    }
}


