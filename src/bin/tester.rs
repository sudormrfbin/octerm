use meow::{components::Component, App, Cmd};
use octerm::{
    components::{IssueView, IssueViewMsg},
    github::{self, events::DateTimeUtc, User},
};

enum Msg {
    IssueViewMsg(IssueViewMsg),
}

struct Model {
    issue: IssueView,
}

struct Tester {}

impl App for Tester {
    type Msg = Msg;

    type Model = Model;

    type Request = ();
    type Response = ();

    fn init() -> Self::Model {
        Model {
            issue: fake_issue().into(),
        }
    }

    fn event_to_msg(event: meow::AppEvent, model: &Self::Model) -> Option<Self::Msg> {
        match event {
            _ => Some(Msg::IssueViewMsg(model.issue.event_to_msg(event)?)),
        }
    }

    fn update(msg: Self::Msg, model: &mut Self::Model) -> meow::Cmd<Self::Request> {
        match msg {
            Msg::IssueViewMsg(IssueViewMsg::CloseView) => Cmd::Quit,
            Msg::IssueViewMsg(msg) => model.issue.update(msg),
        }
    }

    fn view<'m>(model: &'m Self::Model) -> Box<dyn meow::components::Renderable + 'm> {
        Box::new(&model.issue)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    meow::run::<Tester>(None)?;

    Ok(())
}

fn fake_issue() -> github::Issue {
    let date = DateTimeUtc::from_utc(
        chrono::NaiveDate::from_ymd_opt(2022, 4, 12)
            .unwrap()
            .and_hms_opt(3, 45, 23)
            .unwrap(),
        chrono::Utc,
    );
    github::Issue {
        meta: github::IssueMeta {
            repo: github::RepoMeta {
                name: "helix".into(),
                owner: "helix-editor".into(),
            },
            title: "This is the issue title".into(),
            body: r#"The distinction between mouse select and <space>y isn't very clear:
- Mouse: `yanked main selection to system clipboard`
- Space + y: `joined and yanked 1 selection(s) to system clipboard`

It is not clear from this prompt that one of them goes to the middle click clipboard yet the other one goes to the normal (Ctrl+C/V) clipboard. Does anyone know what's the correct term for not only X (which I know are clipboard and primary) but also Wayland? (Maybe we just call them "system clipboard" and "system selection clipboard"? Also, the `joined and yanked 1 selections(s)` is unnecessary. Simply `yanked selection to system clipboard` would do.
"#.into(),
            number: 1045,
            author: User::new("username"),
            state: github::IssueState::Open,
            created_at: date,
        },
        events: vec![
            github::events::EventKind::Commented {
                body: r#"As a workaround you can specify a config for the lsp in the languaguages.toml. 
Example:
```
[[language]]
name = "scala"
scope = "source.scala"
roots = ["build.sbt", "pom.xml"]
file-types = ["scala", "sbt"]
comment-token = "//"
indent = { tab-width = 2, unit = "  " }
language-server = { command = "metals" }
config = {metals.ammoniteJvmProperties = ["-Xmx1G"]}
```
"#.into(),
                }.with(
                User::new("issue-author"), date + chrono::Days::new(1)
            ),
            github::events::EventKind::Commented{
                body: "Just a heads up, we've fixed this in Metals.\
You can test with the latest snapshot to see this working `0.11.9+128-92db24b7-SNAPSHOT`.\
".into(),
                }.with(User::new("replier"), date + chrono::Days::new(2)),
            github::events::EventKind::Commented{
                body: r#"As a workaround you can specify a config for the lsp in the languaguages.toml. 
Example:
```
[[language]]
name = "scala"
scope = "source.scala"
roots = ["build.sbt", "pom.xml"]
file-types = ["scala", "sbt"]
comment-token = "//"
indent = { tab-width = 2, unit = "  " }
language-server = { command = "metals" }
config = {metals.ammoniteJvmProperties = ["-Xmx1G"]}
```
"#.into(),
                }.with(User::new("issue-author"), date + chrono::Days::new(3)),
            ],
        }
}
