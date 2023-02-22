//! Contains parsers that use the parser combinators from
//! [`crate::parsec`] to parse our DSL.

use crate::parsec::*;

use self::types::{Adapter, Command, Consumer, Producer};

pub fn word() -> impl Fn(&str) -> Result<(&str, String), &str> {
    |input: &str| {
        let (rem, chars) = many1(pred(|ch| ch.is_alphanumeric()))(input)?;
        Ok((rem, chars.iter().collect()))
    }
}

pub fn args() -> impl Fn(&str) -> Result<(&str, Vec<String>), &str> {
    |input: &str| {
        let arg = left(and(word(), whitespace0()));
        many0(arg)(input)
    }
}

pub fn uint() -> impl Fn(&str) -> Result<(&str, usize), &str> {
    |input: &str| {
        let (rem, chars) = many1(pred(|ch| ch.is_ascii_digit()))(input)?;
        let uint = chars
            .iter()
            .collect::<String>()
            .parse()
            .map_err(|_| "Could not parse as unsigned integer")?;
        Ok((rem, uint))
    }
}

pub fn uint_args() -> impl Fn(&str) -> Result<(&str, Vec<usize>), &str> {
    |input: &str| {
        let arg = left(and(uint(), whitespace0()));
        many0(arg)(input)
    }
}

/// Parses any of the given literals into an Enum.
pub fn literal_to_enum<E, const N: usize>(
    lits: [&'static str; N],
) -> impl Fn(&str) -> Result<(&str, E), &str>
where
    E: TryFrom<&'static str, Error = &'static str>,
{
    move |input: &str| {
        let lits_parser = lits.map(literal);
        let (input, prod) = any(&lits_parser)(input)?;
        Ok((input, E::try_from(prod)?))
    }
}

pub fn command() -> impl Fn(&str) -> Result<(&str, Command), &str> {
    literal_to_enum(Command::all())
}

pub fn producer() -> impl Fn(&str) -> Result<(&str, Producer), &str> {
    literal_to_enum(Producer::all())
}

pub fn adapter() -> impl Fn(&str) -> Result<(&str, Adapter), &str> {
    literal_to_enum(Adapter::all())
}

pub fn consumer() -> impl Fn(&str) -> Result<(&str, Consumer), &str> {
    literal_to_enum(Consumer::all())
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
}
