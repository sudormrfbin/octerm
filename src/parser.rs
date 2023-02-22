//! Contains parsers that use the parser combinators from
//! [`crate::parsec`] to parse our DSL.

use crate::parsec::*;

use self::types::{
    Adapter, AdapterWithArgs, Command, Consumer, ConsumerWithArgs, Producer, ProducerPipe,
    ProducerWithArgs,
};

pub fn word() -> impl Fn(&str) -> ParseResult<String> {
    let parser = many1(pred(|ch| ch.is_alphanumeric()));
    map(parser, |chars| chars.iter().collect())
}

pub fn args() -> impl Fn(&str) -> ParseResult<Vec<String>> {
    let arg = left(and(word(), whitespace0()));
    many0(arg)
}

pub fn uint() -> impl Fn(&str) -> ParseResult<usize> {
    let parser = many1(pred(|ch| ch.is_ascii_digit()));
    let chars_to_usize = |chars: Vec<char>| chars.iter().collect::<String>().parse().unwrap();
    map(parser, chars_to_usize)
}

pub fn uint_args() -> impl Fn(&str) -> ParseResult<Vec<usize>> {
    let arg = left(and(uint(), whitespace0()));
    many0(arg)
}

/// Parses any of the given literals into an Enum.
pub fn literal_to_enum<E, const N: usize>(
    lits: [&'static str; N],
) -> impl Fn(&str) -> ParseResult<E>
where
    E: TryFrom<&'static str, Error = &'static str>,
{
    move |input: &str| {
        let lits_parser = lits.map(literal);
        let (input, prod) = any(&lits_parser)(input)?;
        Ok((input, E::try_from(prod)?))
    }
}

pub fn command() -> impl Fn(&str) -> ParseResult<Command> {
    literal_to_enum(Command::all())
}

pub fn producer() -> impl Fn(&str) -> ParseResult<Producer> {
    literal_to_enum(Producer::all())
}

pub fn adapter() -> impl Fn(&str) -> ParseResult<Adapter> {
    literal_to_enum(Adapter::all())
}

pub fn consumer() -> impl Fn(&str) -> ParseResult<Consumer> {
    literal_to_enum(Consumer::all())
}

pub fn pipe() -> impl Fn(&str) -> ParseResult<()> {
    map(literal("|"), |_| ())
}

pub fn producer_with_args() -> impl Fn(&str) -> ParseResult<ProducerWithArgs> {
    let maybe_args = maybe(right(and(whitespace1(), args())));
    map(and(producer(), maybe_args), |(producer, args)| {
        ProducerWithArgs {
            producer,
            args: args.unwrap_or_default(),
        }
    })
}

pub fn consumer_with_args() -> impl Fn(&str) -> ParseResult<ConsumerWithArgs> {
    let maybe_args = maybe(right(and(whitespace1(), uint_args())));
    map(and(consumer(), maybe_args), |(consumer, args)| {
        ConsumerWithArgs {
            consumer,
            args: args.unwrap_or_default(),
        }
    })
}

pub fn adapter_with_args() -> impl Fn(&str) -> ParseResult<AdapterWithArgs> {
    let maybe_args = maybe(right(and(whitespace1(), args())));
    map(and(adapter(), maybe_args), |(adapter, args)| {
        AdapterWithArgs {
            adapter,
            args: args.unwrap_or_default(),
        }
    })
}


pub mod types {
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
    pub enum Adapter {}

    impl Adapter {
        pub const fn all() -> [&'static str; 0] {
            []
        }
    }

    impl TryFrom<&str> for Adapter {
        type Error = &'static str;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            match value {
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
    pub struct ProducerPipe {
        pub producer: ProducerWithArgs,
        pub adapters: Vec<AdapterWithArgs>,
        pub consumer: Option<ConsumerWithArgs>,
    }

    pub enum Parsed {
        Command(Command),
        ProducerWithArgs(ProducerWithArgs),
        ProducerPipe(ProducerPipe),
        ConsumerWithArgs(ConsumerWithArgs),
    }
}

#[cfg(test)]

mod test {
    use super::*;

    macro_rules! s {
        ($string:literal) => {
            String::from($string)
        };
    }

    #[test]
    fn test_word() {
        let parse = word();
        assert_eq!(parse("list"), Ok(("", s!("list"))));
        assert_eq!(parse("list pr"), Ok((" pr", s!("list"))));
        assert!(parse("").is_err())
    }

    #[test]
    fn test_args() {
        let parse = args();
        assert_eq!(
            parse("list pr open"),
            Ok(("", vec![s!("list"), s!("pr"), s!("open")]))
        );
        assert_eq!(parse("list pr "), Ok(("", vec![s!("list"), s!("pr")])));
        assert_eq!(
            parse("list pr | open"),
            Ok(("| open", vec![s!("list"), s!("pr")]))
        );
        assert_eq!(
            parse("list pr| open"),
            Ok(("| open", vec![s!("list"), s!("pr")]))
        );
        assert_eq!(parse(""), Ok(("", vec![])));
        assert_eq!(parse("  "), Ok(("  ", vec![])));
    }

    #[test]
    fn test_uint() {
        let parse = uint();
        assert_eq!(parse("124"), Ok(("", 124)));
        assert_eq!(parse("1 | done"), Ok((" | done", 1)));
        assert!(parse("").is_err())
    }

    #[test]
    fn test_uint_args() {
        let parse = uint_args();
        assert_eq!(parse("12 23 345"), Ok(("", vec![12, 23, 345])));
        assert_eq!(parse("12 23 "), Ok(("", vec![12, 23])));
        assert_eq!(parse("12 23 | open"), Ok(("| open", vec![12, 23])));
        assert_eq!(parse("12 23| open"), Ok(("| open", vec![12, 23])));
        assert_eq!(parse(""), Ok(("", vec![])));
        assert_eq!(parse("  "), Ok(("  ", vec![])));
    }

    #[test]
    fn test_command() {
        let parse = command();
        assert_eq!(parse("reload"), Ok(("", Command::Reload)));
        assert!(parse("list").is_err());
    }

    #[test]
    fn test_consumer() {
        let parse = consumer();
        assert_eq!(parse("done"), Ok(("", Consumer::Done)));
        assert_eq!(parse("open"), Ok(("", Consumer::Open)));
        assert_eq!(parse("open 1 2"), Ok((" 1 2", Consumer::Open)));
        assert!(parse("list").is_err());
    }

    #[test]
    fn test_producer_with_args() {
        let parse = producer_with_args();
        let test = |input, prod, args: &[&str], next_input| {
            assert_eq!(
                parse(input),
                Ok((
                    next_input,
                    ProducerWithArgs {
                        producer: prod,
                        args: args.iter().map(ToString::to_string).collect(),
                    }
                ))
            );
        };
        test("list pr open", Producer::List, &["pr", "open"], "");
        test("list pr | done", Producer::List, &["pr"], "| done");
        test("list pr|done", Producer::List, &["pr"], "|done");
        test("list | done", Producer::List, &[], "| done");
        test("list|done", Producer::List, &[], "|done");
        test("list", Producer::List, &[], "");
        // This is expected when using this parser; we handle this case
        // in the producer_expr parser.
        test("listed", Producer::List, &[], "ed");
        // assert!(parse("listed").is_err());
        // assert!(parse("listed pr").is_err());
        // assert!(parse("listed | done").is_err());
        // assert!(parse("listed|done").is_err());
    }

    #[test]
    fn test_consumer_with_args() {
        let parse = consumer_with_args();
        let test = |input, cons, args: &[usize], next_input| {
            assert_eq!(
                parse(input),
                Ok((
                    next_input,
                    ConsumerWithArgs {
                        consumer: cons,
                        args: args.iter().copied().collect(),
                    }
                ))
            );
        };
        test("done 1 12", Consumer::Done, &[1, 12], "");
        test("done", Consumer::Done, &[], "");
        // Fake syntax
        test("open 1 ; done", Consumer::Open, &[1], "; done");
    }
}
