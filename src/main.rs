mod app;
mod error;
mod github;

use std::time::Duration;

use app::App;

use crate::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let app = App::new()?;
    app.run(tick_rate)
}

