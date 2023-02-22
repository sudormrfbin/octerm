//! Contains types emitted by the main parser.

#[derive(Debug, PartialEq)]
pub enum Command {
    Reload,
}

impl Command {
    pub const fn all() -> [&'static str; 1] {
        ["reload"]
    }
}

impl TryFrom<&str> for Command {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "reload" => Ok(Self::Reload),
            _ => Err("not a command"),
        }
    }
}

// ------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum Producer {
    List,
}

impl Producer {
    pub const fn all() -> [&'static str; 1] {
        ["list"]
    }
}

impl TryFrom<&str> for Producer {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "list" => Ok(Self::List),
            _ => Err("not a producer"),
        }
    }
}

// ------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum Adapter {
    Confirm,
}

impl Adapter {
    pub const fn all() -> [&'static str; 1] {
        ["confirm"]
    }
}

impl TryFrom<&str> for Adapter {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "confirm" => Ok(Self::Confirm),
            _ => Err("not an adapter"),
        }
    }
}

// ------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum Consumer {
    Open,
    Done,
}

impl Consumer {
    pub const fn all() -> [&'static str; 2] {
        ["open", "done"]
    }
}

impl TryFrom<&str> for Consumer {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "open" => Ok(Self::Open),
            "done" => Ok(Self::Done),
            _ => Err("not a consumer"),
        }
    }
}

// ------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub struct ProducerWithArgs {
    pub producer: Producer,
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct ConsumerWithArgs {
    pub consumer: Consumer,
    pub args: Vec<usize>,
}

#[derive(Debug, PartialEq)]
pub struct AdapterWithArgs {
    pub adapter: Adapter,
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct ProducerExpr {
    pub producer: ProducerWithArgs,
    pub adapters: Vec<AdapterWithArgs>,
    pub consumer: Option<Consumer>,
}

#[derive(Debug, PartialEq)]
pub enum Parsed {
    Command(Command),
    ProducerExpr(ProducerExpr),
    ConsumerWithArgs(ConsumerWithArgs),
}
