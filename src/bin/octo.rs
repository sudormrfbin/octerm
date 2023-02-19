use octerm::{error::Error, network::methods::open_notification_in_browser};
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

use crossterm::style::Stylize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;

    // Initialise a statically counted instance
    let builder = octocrab::Octocrab::builder().personal_token(token);
    octocrab::initialise(builder)?;

    println!("Syncing notifications");
    let mut notifs = octerm::network::methods::notifications(octocrab::instance()).await?;
    let mut line_editor = Reedline::create();
    let mut prompt = DefaultPrompt::new(DefaultPromptSegment::Empty, DefaultPromptSegment::Empty);

    loop {
        prompt.left_prompt = DefaultPromptSegment::Basic(format!("{} ", notifs.len()));
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("Exiting.");
                break;
            }
            Ok(Signal::Success(cmd)) => match cmd.split_whitespace().collect::<Vec<_>>().as_slice()
            {
                ["list" | "l"] => {
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
                ["reload" | "r"] => {
                    println!("Syncing notifications");
                    notifs = octerm::network::methods::notifications(octocrab::instance()).await?;
                }
                ["open" | "o", args @ ..] => {
                    for idx in args {
                        match idx.parse::<usize>() {
                            Err(_) => {
                                println!("{}: `{idx}` should be a valid index", "Error".red())
                            }
                            Ok(idx) => match notifs.get(idx) {
                                None => println!(
                                    "{}: `{idx}` out of bounds in notifications list",
                                    "Error".red()
                                ),
                                Some(notif) => match open_notification_in_browser(notif).await {
                                    Ok(()) => {}
                                    Err(_) => {
                                        println!("{}: Could not open browser", "Error".red())
                                    }
                                },
                            },
                        };
                    }
                }
                _ => {}
            },
            Err(err) => eprintln!("Error: {err}"),
        }
    }
    Ok(())
}
