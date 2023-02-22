//! Contains parsers that use the parser combinators from
//! [`crate::parsec`] to parse our DSL.

use crate::parsec::*;

use self::types::{
    Adapter, AdapterWithArgs, Command, Consumer, ConsumerWithArgs, Producer, ProducerExpr,
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

pub fn producer_expr() -> impl Fn(&str) -> ParseResult<ProducerExpr> {
    // TODO: Handle whitespace
    let piped_adapter = right(and(pipe(), adapter_with_args()));
    let piped_adapters = many0(piped_adapter);
    let piped_consumer = right(and(pipe(), consumer_with_args()));

    let producer_expr = and(
        and(producer_with_args(), piped_adapters),
        maybe(piped_consumer),
    );
    let producer_expr = eof(producer_expr);
    map(
        producer_expr,
        |((prod_with_args, adap_with_args), cons_with_args)| ProducerExpr {
            producer: prod_with_args,
            adapters: adap_with_args,
            consumer: cons_with_args,
        },
    )
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
        pub consumer: Option<ConsumerWithArgs>,
    }

    pub enum Parsed {
        Command(Command),
        ProducerWithArgs(ProducerWithArgs),
        ProducerPipe(ProducerExpr),
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

    macro_rules! pexpr {
        (
            $prod:ident $prod_args:expr
        ) => {
            pexpr!($prod $prod_args =>)
        };

        (
            $prod:ident $prod_args:expr =>
            $([$adap:ident $adap_args:expr])*
        ) => {
            pexpr!($prod $prod_args => $([$adap $adap_args])* => None)
        };

        (
            $prod:ident $prod_args:expr =>
            $([$adap:ident $adap_args:expr])* =>
            // , between $cons and $cons_args to prevent recursion (Some
            // gets recognized as ident and (ConsumerWithArgs {..}) gets
            // recognized as expr without the ,).
            $cons:ident, $cons_args:expr
        ) => {
            pexpr!($prod $prod_args => $([$adap $adap_args])* => Some(ConsumerWithArgs {
                consumer: Consumer::$cons,
                args: $cons_args.iter().copied().collect()
            }))
        };

        (
            $prod:ident $prod_args:expr =>
            $([$adap:ident $adap_args:expr])* =>
            $cons_expr:expr
        ) => {
            ProducerPipe {
                producer: ProducerWithArgs {
                    producer: Producer::$prod,
                    args: $prod_args.iter().map(ToString::to_string).collect(),
                },
                adapters: vec![$(
                    AdapterWithArgs {
                        adapter: Adapter::$adap,
                        args: $adap_args.iter().map(ToString::to_string).collect()
                    },
                )*],
                consumer: $cons_expr,
            }
        };
    }

    #[test]
    fn test_producer_expr() {
        let parse = producer_expr();

        // An empty [] confuses type inference so annotate the type and pass on to macro
        let a0: [&'static str; 0] = []; // zero args
        let ua0: [usize; 0] = []; // zero usize args

        macro_rules! test {
            ($input:literal, $pexp:expr, $msg:literal) => {
                assert_eq!(parse($input), Ok(("", $pexp)), "{}: {}", $input, $msg)
            };
        }

        assert!(parse("lister").is_err());
        test!("list", pexpr!(List a0), "bare producer");

        test!(
            "list pr open",
            pexpr!(List ["pr", "open"]),
            "producer with args"
        );
        test!(
            "list pr open ",
            pexpr!(List ["pr", "open"]),
            "producer with args with trailing whitespace"
        );
        test!(
            "list|confirm",
            pexpr!(List a0 => [Confirm a0]),
            "bare producer and bare adapter"
        );
        test!(
            "list pr|confirm",
            pexpr!(List ["pr"] => [Confirm a0]),
            "producer with args and bare adapter"
        );
        test!(
            "list pr|confirm smt",
            pexpr!(List ["pr"] => [Confirm ["smt"]]),
            "producer with args and adapter with args"
        );
        test!(
            "list|confirm smt",
            pexpr!(List a0 => [Confirm ["smt"]]),
            "bare producer and adapter with args"
        );
        test!(
            "list|confirm|confirm",
            pexpr!(List a0 => [Confirm a0] [Confirm a0]),
            "bare producer and bare adapter and bare adapter"
        );
        test!(
            "list|confirm|done",
            pexpr!(List a0 => [Confirm a0] => Done, ua0),
            "bare producer and bare adapter and bare consumer"
        );
        test!(
            "list|confirm smt|done",
            pexpr!(List a0 => [Confirm ["smt"]] => Done, ua0),
            "bare producer and adapter with args and bare consumer"
        );
        test!(
            "list|confirm|confirm|done",
            pexpr!(List a0 => [Confirm a0] [Confirm a0] => Done, ua0),
            "bare producer and bare adapter*s* and bare consumer"
        );
    }
}
