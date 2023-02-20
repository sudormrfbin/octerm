use octerm::{error::Error, github::Notification, network::methods::open_notification_in_browser};
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

use crossterm::style::Stylize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;

    // Initialise a statically counted instance
    let builder = octocrab::Octocrab::builder().personal_token(token);
    octocrab::initialise(builder)?;

    println!("Syncing notifications");
    let mut notifications = octerm::network::methods::notifications(octocrab::instance()).await?;
    let mut line_editor = Reedline::create();
    let mut prompt = DefaultPrompt::new(DefaultPromptSegment::Empty, DefaultPromptSegment::Empty);

    loop {
        prompt.left_prompt = DefaultPromptSegment::Basic(format!("{} ", notifications.len()));
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("Exiting.");
                break;
            }
            Ok(Signal::Success(cmd)) => {
                let cmd_result = match cmd.split_whitespace().collect::<Vec<_>>().as_slice() {
                    ["list" | "l"] => list(&notifications).await,
                    ["reload" | "r"] => reload(&mut notifications).await,
                    ["open" | "o", args @ ..] => open(&mut notifications, args).await,
                    _ => Ok(()),
                };

                if let Err(err) = cmd_result {
                    print_error(&err);
                }
            }
            Err(err) => eprintln!("Error: {err}"),
        }
    }
    Ok(())
}

pub async fn list(notifications: &[Notification]) -> Result<(), String> {
    for (i, notif) in notifications.iter().enumerate().rev() {
        println!("{i:2}. {}", notif.to_colored_string());
    }

    Ok(())
}

pub async fn reload(notifications: &mut Vec<Notification>) -> Result<(), String> {
    println!("Syncing notifications");
    *notifications = octerm::network::methods::notifications(octocrab::instance())
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

pub async fn open(notifications: &mut Vec<Notification>, args: &[&str]) -> Result<(), String> {
    let indices: Vec<usize> = args
        .iter()
        .map(|idx| {
            let idx = idx
                .parse::<usize>()
                .map_err(|_| format!("{idx} is not a valid index"))?;
            match idx < notifications.len() {
                true => Ok(idx),
                false => Err(format!("{idx} is out of bounds in list")),
            }
        })
        .collect::<Result<Vec<usize>, String>>()?;
    let futs = indices
        .iter()
        .map(|i| &notifications[*i])
        .map(|notification| open_notification_in_browser(&notification));
    futures::future::join_all(futs)
        .await
        .into_iter()
        .collect::<Result<Vec<()>, Error>>()
        .map_err(|err| format!("Could not open browser: {err}"))?;

    Ok(())
}

pub fn print_error(msg: &str) {
    println!("{}: {msg}", "Error".red())
}
