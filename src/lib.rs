#[cfg(feature = "tui")]
pub mod app;

#[cfg(feature = "tui")]
pub mod components;

pub mod error;
pub mod github;

#[cfg(feature = "tui")]
pub mod markdown;

pub mod network;
pub mod parsec;
pub mod parser;
pub mod util;
