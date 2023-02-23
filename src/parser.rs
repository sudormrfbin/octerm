//! Contains parsers that use the parser combinators from
//! [`crate::parsec`] to parse our DSL.

pub mod types;

use crate::parsec::*;

use self::types::{
    Adapter, AdapterWithArgs, Command, Consumer, ConsumerWithArgs, Parsed, Producer, ProducerExpr,
    ProducerWithArgs,
};

fn word() -> impl Fn(&str) -> ParseResult<String> {
    let parser = many1(pred(|ch| ch.is_alphanumeric()));
    map(parser, |chars| chars.iter().collect())
}

fn args() -> impl Fn(&str) -> ParseResult<Vec<String>> {
    let arg = left(and(word(), whitespace0()));
    many0(arg)
}

fn uint() -> impl Fn(&str) -> ParseResult<usize> {
    let parser = many1(pred(|ch| ch.is_ascii_digit()));
    let chars_to_usize = |chars: Vec<char>| chars.iter().collect::<String>().parse().unwrap();
    map(parser, chars_to_usize)
}

fn uint_args() -> impl Fn(&str) -> ParseResult<Vec<usize>> {
    let arg = left(and(uint(), whitespace0()));
    many0(arg)
}

/// Parses any of the given literals into an Enum.
fn literal_to_enum<E, const N: usize>(lits: [&'static str; N]) -> impl Fn(&str) -> ParseResult<E>
where
    E: TryFrom<&'static str, Error = &'static str>,
{
    move |input: &str| {
        let lits_parser = lits.map(literal);
        let (input, prod) = any(&lits_parser)(input)?;
        Ok((input, E::try_from(prod)?))
    }
}

fn command() -> impl Fn(&str) -> ParseResult<Command> {
    literal_to_enum(Command::all())
}

fn producer() -> impl Fn(&str) -> ParseResult<Producer> {
    literal_to_enum(Producer::all())
}

fn adapter() -> impl Fn(&str) -> ParseResult<Adapter> {
    literal_to_enum(Adapter::all())
}

fn consumer() -> impl Fn(&str) -> ParseResult<Consumer> {
    literal_to_enum(Consumer::all())
}

fn pipe() -> impl Fn(&str) -> ParseResult<()> {
    map(literal("|"), |_| ())
}

fn producer_with_args() -> impl Fn(&str) -> ParseResult<ProducerWithArgs> {
    let maybe_args = maybe(right(and(whitespace1(), args())));
    map(and(producer(), maybe_args), |(producer, args)| {
        ProducerWithArgs {
            producer,
            args: args.unwrap_or_default(),
        }
    })
}

fn consumer_with_args() -> impl Fn(&str) -> ParseResult<ConsumerWithArgs> {
    let maybe_args = maybe(right(and(whitespace1(), uint_args())));
    map(and(consumer(), maybe_args), |(consumer, args)| {
        ConsumerWithArgs {
            consumer,
            args: args.unwrap_or_default(),
        }
    })
}

fn adapter_with_args() -> impl Fn(&str) -> ParseResult<AdapterWithArgs> {
    let maybe_args = maybe(right(and(whitespace1(), args())));
    map(and(adapter(), maybe_args), |(adapter, args)| {
        AdapterWithArgs {
            adapter,
            args: args.unwrap_or_default(),
        }
    })
}

fn producer_expr() -> impl Fn(&str) -> ParseResult<ProducerExpr> {
    // TODO: Handle whitespace
    let piped_adapter = right(and(pipe(), adapter_with_args()));
    let piped_adapters = many0(piped_adapter);
    let piped_consumer = right(and(pipe(), consumer()));

    let producer_expr = and(
        and(producer_with_args(), piped_adapters),
        maybe(piped_consumer),
    );
    map(producer_expr, |((prod_with_args, adap_with_args), cons)| {
        ProducerExpr {
            producer: prod_with_args,
            adapters: adap_with_args,
            consumer: cons,
        }
    })
}

pub fn parser() -> impl Fn(&str) -> ParseResult<Parsed> {
    let command = map(eof(command()), Parsed::Command);
    let prod_expr = map(eof(producer_expr()), Parsed::ProducerExpr);
    let cons_with_args = map(eof(consumer_with_args()), Parsed::ConsumerWithArgs);

    or(or(command, prod_expr), cons_with_args)
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
        // in the top level parser.
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
                        args: args.to_vec(),
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
            $prod:ident $($prod_args:expr)?
            $(=> [$adap:ident $($adap_args:expr)?])*
            $(=> $cons:ident)?
        ) => {
            ProducerExpr {
                producer: ProducerWithArgs {
                    producer: Producer::$prod,
                    args: pexpr!(@maybe_args $($prod_args)?),
                },
                adapters: vec![$(
                    AdapterWithArgs {
                        adapter: Adapter::$adap,
                        args: pexpr!(@maybe_args $($adap_args)?),
                    },
                )*],
                consumer: pexpr!(@optional_conusmer $($cons)?),
            }
        };

        (@maybe_args) => { vec![] };
        (@maybe_args $args:expr) => { $args.iter().map(ToString::to_string).collect() };

        (@optional_conusmer) => { None };
        (@optional_conusmer $val:ident) => { Some(Consumer::$val) };
    }

    #[test]
    fn test_producer_expr() {
        let parse = producer_expr();

        macro_rules! test {
            ($input:literal, $pexp:expr, $msg:literal) => {
                assert_eq!(parse($input), Ok(("", $pexp)), "{}: {}", $input, $msg)
            };
        }

        test!("list", pexpr!(List), "bare producer");
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
            pexpr!(List => [Confirm]),
            "bare producer and bare adapter"
        );
        test!(
            "list pr|confirm",
            pexpr!(List ["pr"] => [Confirm]),
            "producer with args and bare adapter"
        );
        test!(
            "list pr|confirm smt",
            pexpr!(List ["pr"] => [Confirm ["smt"]]),
            "producer with args and adapter with args"
        );
        test!(
            "list|confirm smt",
            pexpr!(List => [Confirm ["smt"]]),
            "bare producer and adapter with args"
        );
        test!(
            "list|confirm|confirm",
            pexpr!(List => [Confirm] => [Confirm]),
            "bare producer and bare adapter and bare adapter"
        );
        test!(
            "list|confirm|done",
            pexpr!(List => [Confirm] => Done),
            "bare producer and bare adapter and bare consumer"
        );
        test!(
            "list|confirm smt|done",
            pexpr!(List => [Confirm ["smt"]] => Done),
            "bare producer and adapter with args and bare consumer"
        );
        test!(
            "list|confirm|confirm|done",
            pexpr!(List => [Confirm] => [Confirm] => Done),
            "bare producer and bare adapter*s* and bare consumer"
        );

        // Expected because eof is not enforced in the producer_expr parser
        // but rather in the top level parser.
        assert_eq!(
            parse("lister"),
            Ok(("er", pexpr!(List))),
            "parses partial input with data remanining in input stream"
        );
    }

    #[test]
    fn test_parser() {
        let parse = parser();

        assert_eq!(parse("reload"), Ok(("", Parsed::Command(Command::Reload))));
        assert_eq!(parse("list"), Ok(("", Parsed::ProducerExpr(pexpr!(List)))));
        assert_eq!(
            parse("list pr|confirm smt|done"),
            Ok((
                "",
                Parsed::ProducerExpr(pexpr!(List ["pr"] => [Confirm ["smt"]] => Done))
            )),
        );
        assert_eq!(
            parse("done 1 2"),
            Ok((
                "",
                Parsed::ConsumerWithArgs(ConsumerWithArgs {
                    consumer: Consumer::Done,
                    args: vec![1, 2]
                })
            ))
        );
        assert!(parse("lister").is_err());
    }
}
