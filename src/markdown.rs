use meow::{
    components::text::{Span, Spans, Text},
    style::{Color, Style, Stylize},
};
use pulldown_cmark::{Event, Options, Tag};

pub fn parse<'a>(source: &'a str) -> Text<'a> {
    let mut tags: Vec<Tag> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut lines: Vec<Spans> = Vec::new();

    let bold_style = Style::default().bold(true).fg(Color::Purple);
    let italic_style = Style::default().italic(true).fg(Color::Magenta);
    let code_style = Style::default().reverse(true);
    let block_code_style = Style::default().bg(Color::Yellow);
    let heading1_style = Style::default()
        .underline(meow::style::Underline::Single)
        .bold(true)
        .fg(Color::White);
    let heading_style = Style::default().underline(meow::style::Underline::Single);
    let html_style = Style::default().dim(true);

    let get_heading_style = |level| match level as usize {
        1 => heading1_style.clone(),
        _ => heading_style.clone(),
    };

    let parser = pulldown_cmark::Parser::new_ext(source, Options::ENABLE_STRIKETHROUGH);

    for event in parser {
        match event {
            Event::Start(tag) => {
                // TODO: handle ordered list with List(Some(idx))
                match tag {
                    Tag::Item => {
                        // list item
                        spans.push(Span::new("• "))
                    }
                    Tag::Heading(level, _, _) => {
                        let mut header = "#".repeat(level as usize);
                        header.push(' ');
                        let style = get_heading_style(level);
                        spans.push(Span::new(header).style(style))
                    }
                    _ => (),
                }
                tags.push(tag);
            }
            Event::End(tag) => {
                tags.pop();
                match tag {
                    Tag::Heading(_, _, _) | Tag::Paragraph | Tag::CodeBlock(_) | Tag::List(_) => {
                        // whenever code block or paragraph closes, new line
                        let spans = std::mem::take(&mut spans);
                        if !spans.is_empty() {
                            lines.push(Spans::new(spans));
                        }
                        lines.push(Spans::default());
                    }
                    Tag::Item => {
                        let spans = std::mem::take(&mut spans);
                        if !spans.is_empty() {
                            lines.push(Spans::new(spans));
                        }
                    }
                    _ => (),
                }
            }
            Event::Text(text) => {
                let tag = tags.last();
                match tag {
                    Some(Tag::Strong) => spans.push(Span::new(text).style(bold_style.clone())),
                    Some(Tag::Emphasis) => spans.push(Span::new(text).style(italic_style.clone())),
                    Some(Tag::Heading(level, _, _)) => {
                        let style = get_heading_style(*level);
                        spans.push(Span::new(text).style(style))
                    }
                    Some(Tag::CodeBlock(_)) => {
                        // line breaks in codeblocks are not reported as events
                        // BUG: In codeblocks, pulldown_cmark does not report line ending events
                        // (HardBreak) and everything inside the block is send as one Text
                        // event. This seems to be the desired behavior, but crlf line
                        // endings cause the text to be broken up as separate Text events
                        // on each newline, but they also have a `\n` at the beginning of
                        // every line. Github uses crlf, so this ends up being a problem for us.
                        let text = text.trim_start_matches('\n');
                        // TODO: append each line to lines vector since a Span is supposed
                        // to last only a single line
                        let span = Span::new(text.to_string()).style(block_code_style.clone());
                        lines.push(Spans::from(span));
                    }
                    Some(Tag::Strikethrough) => spans.push(Span::new(text).strikethrough(true)),
                    Some(_) | None => spans.push(Span::new(text)),
                }
            }
            Event::Code(text) => spans.push(Span::new(text).style(code_style.clone())),
            Event::Html(text) => {
                for line in text.lines() {
                    let span = Span::new(line.to_string()).style(html_style.clone());
                    lines.push(Spans::from(span));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                // TODO: reflow instead ? i.e. push a " " to spans
                let spans = std::mem::take(&mut spans);
                lines.push(Spans::new(spans));
            }
            Event::Rule => {
                lines.push(Spans::from(Span::new("━━━━━━━━━━━━")));
                lines.push(Spans::default());
            }
            _ => {
                log::warn!("unhandled markdown event {:?}", event);
            }
        }
    }

    Text::new(lines)
}
