use std::{error::Error, path::PathBuf};

use meow::{
    components::{
        line::BlankLine,
        scroll::{Scroll, ScrollMsg},
        Component,
    },
    key,
    layout::Constraint,
    server::ServerChannel,
    style::Stylize,
    App, Cmd, FromResponse,
};
use octerm::markdown::Markdown;

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
    use pulldown_cmark::Event;
    let parser = pulldown_cmark::Parser::new(text).into_offset_iter();
    let mut nesting = 1;
    for (event, range) in parser {
        if let Event::End(_) = event {
            nesting -= 2;
        }
        println!("{:<3?}:{:nesting$}{:?}", range, " ", event);
        if let Event::Start(_) = event {
            nesting += 2;
        }
    }
    println!("EOF");
}

const DEFAULT_MD_FILE: &str = "render.md";

struct Model {
    text: Scroll<Markdown<'static>>,
    md_file: Option<PathBuf>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            text: Scroll::new(Markdown::default()),
            md_file: None,
        }
    }
}

impl Model {
    pub fn set(&mut self, file: PathBuf, text: String) {
        self.text = Scroll::new(Markdown::new(text.into()));
        self.md_file = Some(file);
    }
}

enum Msg {
    Quit,
    ReloadCurrentFile,

    ScrollMsg(ScrollMsg),

    Response(Response),
}

impl FromResponse<Response> for Msg {
    fn from_response(response: Response) -> Self {
        Msg::Response(response)
    }
}

enum Request {
    LoadFile(PathBuf),
}

enum Response {
    LoadedFile(PathBuf, String),
}

struct MdRenderApp {}

impl App for MdRenderApp {
    type Msg = Msg;
    type Model = Model;

    type Request = Request;
    type Response = Response;

    fn init() -> Self::Model {
        Model::default()
    }

    fn event_to_msg(event: meow::AppEvent, model: &Self::Model) -> Option<Self::Msg> {
        match event {
            key!('q') => Some(Msg::Quit),
            key!('r') => Some(Msg::ReloadCurrentFile),
            _ => Some(Msg::ScrollMsg(model.text.event_to_msg(event)?)),
        }
    }

    fn update(msg: Self::Msg, model: &mut Self::Model) -> meow::Cmd<Self::Request> {
        match msg {
            Msg::Quit => return Cmd::Quit,
            Msg::Response(r) => match r {
                Response::LoadedFile(path, content) => model.set(path, content),
            },
            Msg::ReloadCurrentFile => {
                if let Some(ref file) = model.md_file {
                    return Cmd::ServerRequest(Request::LoadFile(file.clone()));
                }
            }
            Msg::ScrollMsg(m) => return model.text.update(m),
        };
        Cmd::None
    }

    fn view<'m>(model: &'m Self::Model) -> Box<dyn meow::components::Renderable + 'm> {
        let column = meow::column![
            &model.text,
            BlankLine::horizontal() => Constraint::weak().gte().length(1),
            model.md_file.as_ref().map(|f| f.to_string_lossy().into_owned().reverse(true)).unwrap_or_default() => Constraint::strong().eq().length(1),
        ];

        Box::new(column)
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

    let (server_channel, app_channel) = meow::server::channels::<Request, Response>();
    std::thread::spawn(move || start_server(server_channel));
    app_channel.send_to_server(Request::LoadFile(md_file.into()))?;

    meow::run::<MdRenderApp>(Some(app_channel))?;

    Ok(())
}

fn start_server(channel: ServerChannel<Request, Response>) {
    while let Ok(req) = channel.recv_from_app() {
        match req {
            Request::LoadFile(path) => channel
                .send_to_app(Response::LoadedFile(
                    path.clone(),
                    std::fs::read_to_string(path).unwrap(),
                ))
                .unwrap(),
        };
    }
}
