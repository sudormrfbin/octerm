//! Parser combinators used to parse the custom command line syntax.

pub type ParseResult<'inp, Output> = Result<(&'inp str, Output), &'static str>;

pub fn literal<'a>(lit: &'a str) -> impl Fn(&str) -> ParseResult<&'a str> {
    move |input: &str| {
        input
            .strip_prefix(lit)
            .map(|rem| (rem, lit))
            .ok_or("Could not parse literal")
    }
}

pub fn pred(cond: impl Fn(char) -> bool) -> impl Fn(&str) -> ParseResult<char> {
    move |input: &str| match input.chars().next().filter(|ch| cond(*ch)) {
        Some(ch) => Ok((input.strip_prefix(ch).unwrap(), ch)),
        None => Err("predicate not matched"),
    }
}

pub fn peek(cond: impl Fn(char) -> bool) -> impl Fn(&str) -> ParseResult<char> {
    move |input: &str| match input.chars().next().filter(|ch| cond(*ch)) {
        Some(ch) => Ok((input, ch)),
        None => Err("peek not matched"),
    }
}

pub fn many1<Output>(
    parse: impl Fn(&str) -> ParseResult<Output>,
) -> impl Fn(&str) -> ParseResult<Vec<Output>> {
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

pub fn many0<O>(parse: impl Fn(&str) -> ParseResult<O>) -> impl Fn(&str) -> ParseResult<Vec<O>> {
    move |mut input: &str| {
        let mut output = Vec::new();

        while let Ok((inp, out)) = parse(input) {
            input = inp;
            output.push(out);
        }

        Ok((input, output))
    }
}

pub fn whitespace1() -> impl Fn(&str) -> ParseResult<Vec<char>> {
    many1(pred(|ch| ch.is_whitespace()))
}

pub fn whitespace0() -> impl Fn(&str) -> ParseResult<Vec<char>> {
    many0(pred(|ch| ch.is_whitespace()))
}

pub fn any<Output>(
    parsers: &[impl Fn(&str) -> ParseResult<Output>],
) -> impl Fn(&str) -> ParseResult<Output> + '_ {
    move |input: &str| {
        parsers
            .iter()
            .find_map(|p| p(input).ok())
            .ok_or("Did not match any parser")
    }
}

pub fn and<P1, P2, O1, O2>(p1: P1, p2: P2) -> impl Fn(&str) -> ParseResult<(O1, O2)>
where
    P1: Fn(&str) -> ParseResult<O1>,
    P2: Fn(&str) -> ParseResult<O2>,
{
    move |input: &str| {
        let (input, o1) = p1(input)?;
        let (input, o2) = p2(input)?;
        Ok((input, (o1, o2)))
    }
}

pub fn left<P, O1, O2>(parser: P) -> impl Fn(&str) -> ParseResult<O1>
where
    P: Fn(&str) -> ParseResult<(O1, O2)>,
{
    map(parser, |(o1, _)| o1)
}

pub fn right<P, O1, O2>(parser: P) -> impl Fn(&str) -> ParseResult<O2>
where
    P: Fn(&str) -> ParseResult<(O1, O2)>,
{
    map(parser, |(_, o2)| o2)
}

pub fn map<P, O1, O2>(parser: P, f: impl Fn(O1) -> O2) -> impl Fn(&str) -> ParseResult<O2>
where
    P: Fn(&str) -> ParseResult<O1>,
{
    move |input: &str| {
        let (rem, o1) = parser(input)?;
        Ok((rem, f(o1)))
    }
}

pub fn maybe<P, O>(parser: P) -> impl Fn(&str) -> ParseResult<Option<O>>
where
    P: Fn(&str) -> ParseResult<O>,
{
    move |input: &str| match parser(input) {
        Ok((next_input, output)) => Ok((next_input, Some(output))),
        Err(_) => Ok((input, None)),
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
    fn test_peek() {
        let parse = peek(|ch| ch.is_ascii_digit());
        assert_eq!(parse("123"), Ok(("123", '1')));
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

    #[test]
    fn test_right() {
        let pipe_and_sort = and(literal("|"), literal("sort"));
        let parse = and(right(pipe_and_sort), literal("|open"));
        assert_eq!(parse("|sort|open"), Ok(("", ("sort", "|open"))));
    }

    #[test]
    fn test_map() {
        let parse = map(whitespace1(), |v| v.len());
        assert_eq!(parse("   "), Ok(("", 3)));
    }

    #[test]
    fn test_maybe() {
        let parse = maybe(literal("wow"));
        assert_eq!(parse("wow"), Ok(("", Some("wow"))));
        assert_eq!(parse("ow"), Ok(("ow", None)));
    }
}
