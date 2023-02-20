use octerm::{
    error::Error,
    github::{Notification, NotificationTarget},
    network::methods::open_notification_in_browser,
};
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
                    ["list" | "l", args @ ..] => list(&notifications, args).await,
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

pub async fn list(notifications: &[Notification], args: &[&str]) -> Result<(), String> {
    // TODO: Robust parsing (invalid tokens, etc)

    let is_pr = args.contains(&"pr");
    let is_issue = args.contains(&"issue");
    let is_closed = args.contains(&"closed");
    let is_open = args.contains(&"open");
    let is_merged = args.contains(&"merged");
    let is_release = args.contains(&"release");
    let is_discussion = args.contains(&"discussion");

    if true_count(&[is_pr, is_issue, is_release, is_discussion]) > 1 {
        return Err("pr, issue, discussion, release are mutually exclusive".to_string());
    }

    if true_count(&[is_open, is_closed, is_merged]) > 1 {
        return Err("pr, issue, merged are mutually exclusive".to_string());
    }

    let filter_by_type = |n: &Notification| -> bool {
        if is_pr {
            matches!(n.target, NotificationTarget::PullRequest(_))
        } else if is_issue {
            matches!(n.target, NotificationTarget::Issue(_))
        } else if is_release {
            matches!(n.target, NotificationTarget::Release(_))
        } else if is_discussion {
            matches!(n.target, NotificationTarget::Discussion(_))
        } else {
            true
        }
    };

    let filter_by_state = |n: &Notification| -> bool {
        if is_open {
            match n.target {
                NotificationTarget::Issue(ref issue) => issue.state.is_open(),
                NotificationTarget::PullRequest(ref pr) => pr.state.is_open(),
                _ => false,
            }
        } else if is_closed {
            match n.target {
                NotificationTarget::Issue(ref issue) => issue.state.is_closed(),
                NotificationTarget::PullRequest(ref pr) => pr.state.is_closed(),
                _ => false,
            }
        } else if is_merged {
            match n.target {
                NotificationTarget::PullRequest(ref pr) => pr.state.is_merged(),
                _ => false,
            }
        } else {
            true
        }
    };

    let list = notifications
        .iter()
        .enumerate()
        .filter(|(_, n)| filter_by_type(n))
        .filter(|(_, n)| filter_by_state(n));

    for (i, notif) in list.rev() {
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

fn true_count(bools: &[bool]) -> usize {
    bools.into_iter().map(|b| *b as usize).sum()
}
