//! Contains parsers that use the parser combinators from
//! [`crate::parsec`] to parse our DSL.

use crate::parsec::*;

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
}
