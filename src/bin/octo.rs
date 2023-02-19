use octerm::error::Error;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

use crossterm::style::Stylize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;

    // Initialise a statically counted instance
    let builder = octocrab::Octocrab::builder().personal_token(token);
    octocrab::initialise(builder)?;

    println!("Syncing notifications");
    let notifs = octerm::network::methods::notifications(octocrab::instance()).await?;
    let mut line_editor = Reedline::create();
    let mut prompt = DefaultPrompt::new(DefaultPromptSegment::Empty, DefaultPromptSegment::Empty);

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("Exiting.");
                break;
            }
            Ok(Signal::Success(cmd)) => match cmd.as_str() {
                "list" | "l" => {
                    for notif in notifs.iter().take(10) {
                        let color = octerm::util::notif_target_color(&notif.target).into();
                        println!(
                            "{repo}: {icon} {title}",
                            repo = notif.inner.repository.name,
                            icon = notif.target.icon().with(color),
                            title = notif.inner.subject.title.as_str().with(color),
                        )
                    }
                    println!("10/{len}...", len = notifs.len());
                }
                _ => {}
            },
            Err(err) => eprintln!("Error: {err}"),
        }

        prompt.left_prompt = DefaultPromptSegment::Basic(format!("{} ", notifs.len()));
    }
    Ok(())
}
