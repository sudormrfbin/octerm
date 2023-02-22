//! Parser combinators used to parse the custom command line syntax.

pub fn literal<'a>(lit: &'a str) -> impl Fn(&str) -> Result<(&str, &'a str), &str> {
    move |input: &str| {
        input
            .strip_prefix(lit)
            .map(|rem| (rem, lit))
            .ok_or("Could not parse literal")
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

pub fn many0<Output>(
    parse: impl Fn(&str) -> Result<(&str, Output), &str>,
) -> impl Fn(&str) -> Result<(&str, Vec<Output>), &str> {
    move |mut input: &str| {
        let mut output = Vec::new();

        while let Ok((inp, out)) = parse(input) {
            input = inp;
            output.push(out);
        }

        Ok((input, output))
    }
}

pub fn whitespace1() -> impl Fn(&str) -> Result<(&str, Vec<char>), &str> {
    move |input: &str| many1(pred(|ch| ch.is_whitespace()))(input)
}

pub fn whitespace0() -> impl Fn(&str) -> Result<(&str, Vec<char>), &str> {
    move |input: &str| many0(pred(|ch| ch.is_whitespace()))(input)
}

pub fn any<Output>(
    parsers: &[impl Fn(&str) -> Result<(&str, Output), &str>],
) -> impl Fn(&str) -> Result<(&str, Output), &str> + '_ {
    move |input: &str| {
        parsers
            .iter()
            .find_map(|p| p(input).ok())
            .ok_or("Did not match any parser")
    }
}

pub fn and<P1, P2, O1, O2>(p1: P1, p2: P2) -> impl Fn(&str) -> Result<(&str, (O1, O2)), &str>
where
    P1: Fn(&str) -> Result<(&str, O1), &str>,
    P2: Fn(&str) -> Result<(&str, O2), &str>,
{
    move |input: &str| {
        let (input, o1) = p1(input)?;
        let (input, o2) = p2(input)?;
        Ok((input, (o1, o2)))
    }
}

pub fn left<P, O1, O2>(parser: P) -> impl Fn(&str) -> Result<(&str, O1), &str>
where
    P: Fn(&str) -> Result<(&str, (O1, O2)), &str>,
{
    move |input: &str| {
        let (input, (o1, _)) = parser(input)?;
        Ok((input, o1))
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
        let parse = pred(|ch| ch.is_ascii_digit());
        assert_eq!(parse("123"), Ok(("23", '1')));
        assert!(parse("a12").is_err());
    }

    #[test]
    fn test_many1() {
        let parse = many1(pred(|ch| ch.is_ascii_digit()));
        assert_eq!(parse("123"), Ok(("", vec!['1', '2', '3'])));
        assert_eq!(parse("12q3"), Ok(("q3", vec!['1', '2'])));
        assert!(parse("q3").is_err());
    }

    #[test]
    fn test_many0() {
        let parse = many0(pred(|ch| ch.is_ascii_digit()));
        assert_eq!(parse("123"), Ok(("", vec!['1', '2', '3'])));
        assert_eq!(parse("12q3"), Ok(("q3", vec!['1', '2'])));
        assert_eq!(parse("q3"), Ok(("q3", vec![])));
    }

    #[test]
    fn test_whitespace1() {
        let parse = whitespace1();
        assert_eq!(parse("  1"), Ok(("1", vec![' ', ' '])));
        assert_eq!(parse("  "), Ok(("", vec![' ', ' '])));
        assert_eq!(parse("\n "), Ok(("", vec!['\n', ' '])));
        assert!(parse("q").is_err());
    }

    #[test]
    fn test_whitespace0() {
        let parse = whitespace0();
        assert_eq!(parse("  1"), Ok(("1", vec![' ', ' '])));
        assert_eq!(parse("  "), Ok(("", vec![' ', ' '])));
        assert_eq!(parse("\n "), Ok(("", vec!['\n', ' '])));
        assert_eq!(parse("q"), Ok(("q", vec![])));
    }

    #[test]
    fn test_any() {
        let cmds = [literal("open"), literal("done")];
        let parse = any(&cmds);
        assert_eq!(parse("open 1 2"), Ok((" 1 2", "open")));
        assert_eq!(parse("done"), Ok(("", "done")));
        assert!(parse("list").is_err());
    }

    #[test]
    fn test_and() {
        let parse = and(literal("list"), whitespace1());
        assert_eq!(parse("list  "), Ok(("", ("list", vec![' ', ' ']))));
        let parse = and(and(literal("list"), whitespace1()), literal("pr"));
        assert_eq!(parse("list pr"), Ok(("", (("list", vec![' ']), "pr"))));
    }

    #[test]
    fn test_left() {
        let list_and_whitespace = and(literal("list"), whitespace1());
        let parse = and(left(list_and_whitespace), literal("pr"));
        assert_eq!(parse("list pr"), Ok(("", ("list", "pr"))));
    }
}
