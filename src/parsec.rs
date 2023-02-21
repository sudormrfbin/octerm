//! Parser combinators used to parse the custom command line syntax.

pub fn literal(lit: &'static str) -> impl Fn(&str) -> Result<(&str, &'static str), &str> {
    move |input: &str| {
        input
            .strip_prefix(lit)
            .map(|rem| (rem, lit))
            .ok_or_else(|| "Could not parse literal")
    }
}

pub fn pred(cond: impl Fn(char) -> bool) -> impl Fn(&str) -> Result<(&str, char), &str> {
    move |input: &str| match input.chars().next().filter(|ch| cond(*ch)) {
        Some(ch) => Ok((input.strip_prefix(ch).unwrap(), ch)),
        None => Err("predicate not matched"),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_literal() {
        let parse = literal("list");
        assert_eq!(parse("list pr open"), Ok((" pr open", "list")));
        assert!(parse("open 1 2").is_err());
        assert!(parse("lis").is_err());
    }

    #[test]
    fn test_pred() {
        let parse = pred(|ch| ch.is_digit(10));
        assert_eq!(parse("123"), Ok(("23", '1')));
        assert!(parse("a12").is_err());
    }
}
