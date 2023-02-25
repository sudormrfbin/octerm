use octerm::{
    error::Error,
    github::{Notification, NotificationTarget},
    line_editor,
    parser::types::{
        Adapter, Command, Consumer, ConsumerWithArgs, Parsed, Producer, ProducerExpr,
        ProducerWithArgs,
    },
};
use reedline::Signal;

use crossterm::style::Stylize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;

    // Initialise a statically counted instance
    let builder = octocrab::Octocrab::builder().personal_token(token);
    octocrab::initialise(builder)?;

    println!("Syncing notifications");
    // TODO: Retry in case of bad connection, better error handling, etc.
    let mut notifications = octerm::network::methods::notifications(octocrab::instance()).await?;
    let mut line_editor = line_editor::line_editor();

    loop {
        let sig = line_editor.read_line(&line_editor::prompt(notifications.len()));
        match sig {
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("Exiting.");
                break;
            }
            Ok(Signal::Success(cmdline)) => match octerm::parser::parse(cmdline.trim()) {
                Ok((rem_input, parsed)) => {
                    if !rem_input.is_empty() {
                        print_error(&format!("Invalid expression tail: `{rem_input}`"));
                        continue;
                    }
                    if let Err(err) = run(parsed, &mut notifications).await {
                        print_error(&err);
                    }
                }
                Err(_) => {
                    print_error("Invalid expression");
                    continue;
                }
            },
            Err(err) => print_error(&err.to_string()),
        }
    }
    Ok(())
}

type ExecResult = Result<(), String>;

async fn run(parsed: Parsed, notifications: &mut Vec<Notification>) -> ExecResult {
    match parsed {
        Parsed::Command(cmd) => run_command(cmd, notifications).await?,
        Parsed::ProducerExpr(pexpr) => run_producer_expr(pexpr, notifications).await?,
        Parsed::ConsumerWithArgs(cons) => run_consumer(cons, notifications).await?,
    };
    Ok(())
}

async fn run_command(cmd: Command, notifications: &mut Vec<Notification>) -> ExecResult {
    match cmd {
        Command::Reload => reload(notifications).await?,
    };
    Ok(())
}

async fn run_producer_expr(
    pexpr: ProducerExpr,
    notifications: &mut Vec<Notification>,
) -> ExecResult {
    let ProducerExpr {
        producer:
            ProducerWithArgs {
                producer,
                args: producer_args,
            },
        adapters,
        consumer,
    } = pexpr;

    let mut indices = match producer {
        Producer::List => list(notifications, producer_args).await?,
    };

    for adapter in adapters {
        indices = match adapter.adapter {
            Adapter::Confirm => adapters::confirm(notifications, &indices).await?,
        }
    }

    match consumer {
        None => print_notifications(notifications, &indices),
        Some(consumer) => {
            run_consumer(
                ConsumerWithArgs {
                    consumer,
                    args: indices,
                },
                notifications,
            )
            .await?
        }
    };

    Ok(())
}

async fn run_consumer(cons: ConsumerWithArgs, notifications: &mut Vec<Notification>) -> ExecResult {
    let ConsumerWithArgs {
        consumer: cons,
        args,
    } = cons;

    // TODO: Decide behaviour on empty args
    match cons {
        Consumer::Count => consumers::count(notifications, &args).await?,
        Consumer::Open => consumers::open(notifications, &args).await?,
        Consumer::Done => {
            consumers::done(notifications, &args).await?;
            // Print the list again since done will change the indices
            // let indices = list(notifications, Vec::new()).await?;
            // print_notifications(notifications, &indices);
        }
    };

    Ok(())
}

pub async fn list(notifications: &[Notification], args: Vec<String>) -> Result<Vec<usize>, String> {
    // TODO: Robust parsing (invalid tokens, etc)

    let has_arg = |arg| args.iter().any(|a| *a == arg);
    let is_pr = has_arg("pr");
    let is_issue = has_arg("issue");
    let is_closed = has_arg("closed");
    let is_open = has_arg("open");
    let is_merged = has_arg("merged");
    let is_release = has_arg("release");
    let is_discussion = has_arg("discussion");

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

    let notification_indices = notifications
        .iter()
        .enumerate()
        .filter(|(_, n)| filter_by_type(n))
        .filter(|(_, n)| filter_by_state(n))
        .map(|(i, _)| i)
        .collect();

    Ok(notification_indices)
}

pub async fn reload(notifications: &mut Vec<Notification>) -> Result<(), String> {
    println!("Syncing notifications");
    *notifications = octerm::network::methods::notifications(octocrab::instance())
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

pub mod adapters {
    use std::io::Write;

    use octerm::github::Notification;

    use crate::{format_colored_notification, read_char};

    pub async fn confirm(
        notifications: &[Notification],
        filter: &[usize],
    ) -> Result<Vec<usize>, String> {
        crossterm::terminal::enable_raw_mode().map_err(|_| "Could not enable terminal raw mode")?;

        let result = confirm_helper(notifications, filter);

        // TODO: Register panic handler to always disable raw mode
        crossterm::terminal::disable_raw_mode()
            .map_err(|_| "Could not disable terminal raw mode")?;

        if result.is_err() {
            // Reset cursor to beginning of line
            println!("\r");
        }

        result
    }

    fn confirm_helper(
        notifications: &[Notification],
        filter: &[usize],
    ) -> Result<Vec<usize>, String> {
        let flush = || {
            std::io::stdout()
                .flush()
                .map_err(|_| "Could not flush output")
        };

        let mut indices = Vec::new();

        let mut it = filter.iter().map(|i| (*i, &notifications[*i]));
        let mut next_notification = it.next();

        while let Some((i, notification)) = next_notification {
            print!("{}: [y/n] ", format_colored_notification(i, notification));
            flush()?;
            let mut is_valid_input = true;

            // TODO: Add undo
            // TODO: Add show rest
            let input = read_char().map_err(|_| "Couldn't read input")?;
            print!("{}", input);
            flush()?;

            // Keybindings have been modeled after git add -p
            // TODO: Add additional confirmation keybind for d and a
            // (cannot undo if pressed by accident)?
            match input {
                'y' => indices.push(i),
                'n' => {}
                // Skip this notification and all the remaining ones
                'd' => break,
                // Confirm current notification and all the remaining ones
                'a' => {
                    indices.push(i);
                    while let Some((i, _)) = it.next() {
                        indices.push(i);
                    }
                    break;
                }
                'Q' => return Err("Aborted confirm queue".to_string()),
                _invalid_input => {
                    print!(" (invalid option)");
                    is_valid_input = false;
                }
            }

            // Reset cursor to beginning of line
            println!("\r");

            if is_valid_input {
                next_notification = it.next();
            }
        }

        Ok(indices)
    }
}

pub mod consumers {
    use futures::TryFutureExt;
    use octerm::{
        error::Error,
        github::Notification,
        network::methods::{mark_notification_as_read, open_notification_in_browser},
    };

    pub async fn count(
        _notifications: &mut [Notification],
        filter: &[usize],
    ) -> Result<(), String> {
        println!("{}", filter.len());
        Ok(())
    }

    pub async fn open(notifications: &mut [Notification], filter: &[usize]) -> Result<(), String> {
        let futs = filter
            .iter()
            .map(|i| &notifications[*i])
            .map(open_notification_in_browser);
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
        let has_error = marked.iter().any(|m| m.is_err());
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

fn read_char() -> crossterm::Result<char> {
    use crossterm::event::{Event, KeyCode, KeyEvent /*, KeyModifiers */};

    loop {
        if let Event::Key(event) = crossterm::event::read()? {
            let KeyEvent { code, modifiers } = event;
            if modifiers.is_empty() {
                if let KeyCode::Char(ch) = code {
                    return Ok(ch);
                }
            }
        }
    }
}

fn print_notifications(notifications: &[Notification], indices: &[usize]) {
    for i in indices {
        match notifications.get(*i) {
            Some(n) => println!("{}", format_colored_notification(*i, n)),
            None => print_error("Invalid notifications list index"),
        }
    }
}

fn format_colored_notification(index: usize, notification: &Notification) -> String {
    format!("{index:2}. {}", notification.to_colored_string())
}

fn print_error(msg: &str) {
    println!("{}: {msg}", "Error".red())
}

fn true_count(bools: &[bool]) -> usize {
    bools.iter().map(|b| *b as usize).sum()
}
