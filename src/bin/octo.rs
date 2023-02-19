use octerm::error::Error;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;

    // Initialise a statically counted instance
    let builder = octocrab::Octocrab::builder().personal_token(token);
    octocrab::initialise(builder)?;

    println!("Syncing notifications");
    let notifs = octerm::network::methods::notifications(octocrab::instance()).await?;
    let mut line_editor = Reedline::create();
    let prompt = DefaultPrompt::new(DefaultPromptSegment::Empty, DefaultPromptSegment::Empty);

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("\nAborted!");
                break;
            }
            Ok(Signal::Success(cmd)) => match cmd.as_str() {
                "list" | "l" => notifs
                    .iter()
                    .take(10)
                    .for_each(|n| println!("{}", n.inner.subject.title)),
                _ => {}
            },
            Err(err) => eprintln!("Error: {err}"),
        }
    }
    Ok(())
}
