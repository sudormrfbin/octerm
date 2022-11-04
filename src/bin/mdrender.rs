/*
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CSEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    widgets::Paragraph,
    Frame, Terminal,
};

use octerm::markdown;

const HELP_TEXT: &str = r#"
Usage: mdrender [OPTIONS]

    -f FILE         Take markdown input from FILE. Must come before -e
    -e, --events    Print pulldown-cmark events and exit
    -h, --help      Display help

Renders the markdown given in a file (by default render.md in the CWD)
to the terminal, using octerm's markdown rendering locgic. Useful for
quick testing.

Press "q" to quit the screen and "r" to reload from the file.
"#;

fn print_events(text: &str) {
    let parser = pulldown_cmark::Parser::new(text).into_offset_iter();
    for (event, range) in parser {
        println!("{:<3?}: {:?}", range, event);
    }
    println!("EOF");
}

const DEFAULT_MD_FILE: &str = "render.md";

struct App {
    text: String,
    md_file: String,
}

impl App {
    fn new(md_file: String) -> App {
        let mut app = App {
            md_file,
            text: String::new(),
        };
        app.reload_text();
        app
    }

    fn reload_text(&mut self) {
        let text = std::fs::read_to_string(&self.md_file).unwrap();
        self.text = text;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut md_file = DEFAULT_MD_FILE.to_string();

    let mut args = std::env::args();
    args.next(); // skip program name

    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "-f" => md_file = args.next().expect("-f requires a file path"),
            "-e" | "--events" => {
                print_events(&std::fs::read_to_string(&md_file).unwrap());
                return Ok(());
            }
            "-h" | "--help" => {
                print!("{HELP_TEXT}");
                return Ok(());
            }
            _ => {
                eprintln!("Invalid argument {arg}");
                return Ok(());
            }
        }
    }

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new(md_file);
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let CSEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('r') => app.reload_text(),
                    _ => (),
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let para = Paragraph::new(markdown::parse(&app.text));

    f.render_widget(para, f.size());
}
*/

fn main() {}
