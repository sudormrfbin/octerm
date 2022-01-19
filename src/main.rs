mod app;
mod error;
mod github;

use std::time::Duration;

use app::App;
use octocrab::Octocrab;

use crate::error::{Error, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;
    let octocrab_ = Octocrab::builder()
        .personal_token(token.to_string())
        .build()?;
    let app = App::new(&octocrab_)?;
    app.run(tick_rate)
}
