use octerm::{
    error::Error,
    github::{Notification, NotificationTarget},
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
                    ["open" | "o", args @ ..] => open(&mut notifications, args, None).await,
                    ["done" | "d", args @ ..] => {
                        let result = done(&mut notifications, args, None).await;
                        // Print the list again since done will change the indices
                        let _ = list(&notifications, &[]).await;
                        result
                    }
                    _ => Err("Invalid command".to_string()),
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

pub async fn open(
    notifications: &mut Vec<Notification>,
    args: &[&str],
    piped: Option<Vec<usize>>,
) -> Result<(), String> {
    let indices = arg_or_pipe_indices(args, piped, notifications.len())?;
    consumers::open(notifications, &indices).await
}

pub async fn done(
    notifications: &mut Vec<Notification>,
    args: &[&str],
    piped: Option<Vec<usize>>,
) -> Result<(), String> {
    let indices = arg_or_pipe_indices(args, piped, notifications.len())?;
    consumers::done(notifications, &indices).await
}

pub mod consumers {
    use futures::TryFutureExt;
    use octerm::{
        error::Error,
        github::Notification,
        network::methods::{mark_notification_as_read, open_notification_in_browser},
    };

    pub async fn open(
        notifications: &mut Vec<Notification>,
        filter: &[usize],
    ) -> Result<(), String> {
        let futs = filter
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

    pub async fn done(
        notifications: &mut Vec<Notification>,
        filter: &[usize],
    ) -> Result<(), String> {
        let octo = octocrab::instance();
        let futs = filter
            .iter()
            .map(|i| (i, &notifications[*i]))
            .map(|(i, notification)| {
                mark_notification_as_read(&octo, notification.inner.id).map_ok(|_| *i)
            });
        let marked = futures::future::join_all(futs).await;
        let has_error = marked.iter().find(|m| m.is_err()).is_some();
        let mut marked: Vec<usize> = marked.into_iter().filter_map(|m| m.ok()).collect();
        marked.sort();

        for idx in marked.iter().rev() {
            // Remove from the end so that indices stay stable as items are removed.
            notifications.remove(*idx);
        }

        if has_error {
            return Err("Some notifications could not be marked as read".to_string());
        }

        Ok(())
    }
}

/// For commands that accept both arguments and piped values, return the set
/// of indices that they should act on. Arguments and piped values are mutually
/// exclusive.
fn arg_or_pipe_indices(
    args: &[&str],
    piped: Option<Vec<usize>>,
    list_len: usize,
) -> Result<Vec<usize>, String> {
    let indices = match (args, piped) {
        ([], None) => return Err("No data recieved as arguments or from pipe".to_string()),
        ([_, ..], Some(_)) => {
            // Both args and piped values given
            return Err("Cannot take arguments when used at end of pipe".to_string());
        }
        (args @ [_, ..], None) => validate_indices(args, list_len)?,
        ([], Some(filter)) => filter,
    };
    Ok(indices)
}

/// Convert a list of strings to a list of indices, ensuring that each is
/// a valid index.
fn validate_indices(args: &[&str], list_len: usize) -> Result<Vec<usize>, String> {
    args.iter()
        .map(|idx| {
            let idx = idx
                .parse::<usize>()
                .map_err(|_| format!("{idx} is not a valid index"))?;
            match idx < list_len {
                true => Ok(idx),
                false => Err(format!("{idx} is out of bounds in list")),
            }
        })
        .collect()
}

pub fn print_error(msg: &str) {
    println!("{}: {msg}", "Error".red())
}

fn true_count(bools: &[bool]) -> usize {
    bools.into_iter().map(|b| *b as usize).sum()
}
