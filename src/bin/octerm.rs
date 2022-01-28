use std::{
    error::Error as StdError,
    sync::{mpsc::Receiver, Arc},
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use octocrab::Octocrab;
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use octerm::app::App;
use octerm::error::{Error, Result};
use octerm::events::NotifEvent;
use octerm::network::Network;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let tick_rate = Duration::from_millis(100);
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;

    // This initialises a statically counted instance, which is handy
    // when doing concurrent requests based on tokio tasks.
    let builder = Octocrab::builder().personal_token(token.clone());
    octocrab::initialise(builder)?;

    let builder = Octocrab::builder().personal_token(token.clone());
    let octocrab_ = builder.build()?;

    let (event_tx, event_rx) = std::sync::mpsc::channel::<NotifEvent>();

    let app = Arc::new(Mutex::new(App::new(event_tx)));
    let cloned_app = Arc::clone(&app);

    std::thread::spawn(move || {
        let mut network = Network::new(octocrab_, cloned_app);
        start_async_network_io(event_rx, &mut network);
    });
    start_ui(app, tick_rate).await?;

    Ok(())
}

#[tokio::main]
async fn start_async_network_io(event_rx: Receiver<NotifEvent>, network: &mut Network) {
    while let Ok(event) = event_rx.recv() {
        if let Err(err) = network.handle_event(event).await {
            log::error!("Network error: {:?}", err.source().unwrap_or(&err));
            let mut app = network.app.lock().await;
            app.state
                .statusline
                .set(&format!("Network error: {err}"), "error");
        }
        let mut app = network.app.lock().await;
        app.state.is_loading = false;
    }
}

async fn start_ui(app: Arc<Mutex<App>>, tick_rate: Duration) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    run_event_loop(app, &mut terminal, tick_rate).await?;

    // TODO: Add custom panic handler to restore terminal state
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}

async fn run_event_loop<B: Backend>(
    app: Arc<Mutex<App>>,
    terminal: &mut Terminal<B>,
    tick_rate: Duration,
) -> Result<()> {
    let mut last_tick = Instant::now();
    {
        let mut app = app.lock().await;
        if let Err(err) = app.dispatch_event(NotifEvent::Refresh) {
            app.state.statusline.set(&err, "error");
        }
    }

    loop {
        let mut app = app.lock().await;
        terminal.draw(|f| octerm::ui::draw_ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                app.state.statusline.clear();

                let result = match key.code {
                    KeyCode::Char(c) => app.on_key(c),
                    KeyCode::Enter => app.on_enter(),
                    _ => Ok(()),
                };
                if let Err(err) = result {
                    app.state.statusline.set(&err, "error");
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            if let Err(err) = app.on_tick() {
                app.state.statusline.set(&err, "error");
            }
            last_tick = Instant::now();
        }

        if app.state.should_quit {
            return Ok(());
        }
    }
}
