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

pub fn many1<Output>(
    parse: impl Fn(&str) -> Result<(&str, Output), &str>,
) -> impl Fn(&str) -> Result<(&str, Vec<Output>), &str> {
    move |input: &str| {
        let mut output = Vec::new();
        let (mut input, out) = parse(input)?;
        output.push(out);

        while let Ok((inp, out)) = parse(input) {
            input = inp;
            output.push(out);
        }

        Ok((input, output))
    }
}

pub fn whitespace() -> impl Fn(&str) -> Result<(&str, Vec<char>), &str> {
    move |input: &str| many1(pred(|ch| ch.is_whitespace()))(input)
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

    #[test]
    fn test_many1() {
        let parse = many1(pred(|ch| ch.is_digit(10)));
        assert_eq!(parse("123"), Ok(("", vec!['1', '2', '3'])));
        assert_eq!(parse("12q3"), Ok(("q3", vec!['1', '2'])));
        assert!(parse("q3").is_err());
    }

    #[test]
    fn test_whitespace() {
        let parse = whitespace();
        assert_eq!(parse("  1"), Ok(("1", vec![' ', ' '])));
        assert_eq!(parse("  "), Ok(("", vec![' ', ' '])));
        assert_eq!(parse("\n "), Ok(("", vec!['\n', ' '])));
        assert!(parse("q").is_err());
    }
}
