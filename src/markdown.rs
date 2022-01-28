use pulldown_cmark::{Event, Tag};
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
};

pub fn parse<'a>(source: &'a str) -> Text {
    let mut tags: Vec<Tag> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut lines: Vec<Spans> = Vec::new();

    let bold_style = Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::Cyan);
    let italic_style = Style::default()
        .add_modifier(Modifier::ITALIC)
        .fg(Color::Magenta);
    let code_style = Style::default().add_modifier(Modifier::REVERSED);
    let block_code_style = Style::default().bg(Color::Rgb(62, 68, 82));

    let parser = pulldown_cmark::Parser::new(source);

    for event in parser {
        match event {
            Event::Start(tag) => {
                // TODO: handle ordered list with List(Some(idx))
                match tag {
                    Tag::Item => {
                        // list item
                        spans.push(Span::raw("• "))
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
                            lines.push(Spans::from(spans));
                        }
                        lines.push(Spans::default());
                    }
                    Tag::Item => {
                        let spans = std::mem::take(&mut spans);
                        if !spans.is_empty() {
                            lines.push(Spans::from(spans));
                        }
                    }
                    _ => (),
                }
            }
            Event::Text(text) => {
                let tag = tags.last();
                match tag {
                    Some(Tag::Strong) => spans.push(Span::styled(text, bold_style)),
                    Some(Tag::Emphasis) => spans.push(Span::styled(text, italic_style)),
                    Some(Tag::CodeBlock(_)) => {
                        // line breaks in codeblocks are not reported as events
                            // BUG: In codeblocks, pulldown_cmark does not report line ending events
                            // (HardBreak) and everything inside the block is send as one Text
                            // event. This seems to be the desired behavior, but crlf line
                            // endings cause the text to be broken up as separate Text events
                            // on each newline, but they also have a `\n` at the beginning of
                            // every line. Github uses crlf, so this ends up being a problem for us.
                            let text = text.trim_start_matches('\n');
                            let span = Span::styled(text.to_string(), block_code_style);
                            lines.push(Spans::from(span));
                    }
                    Some(_) | None => spans.push(Span::raw(text)),
                }
            }
            Event::Code(text) => spans.push(Span::styled(text, code_style)),
            Event::Html(text) => spans.push(Span::raw(text)),
            Event::SoftBreak | Event::HardBreak => {
                // TODO: reflow instead ? i.e. push a " " to spans
                let spans = std::mem::take(&mut spans);
                lines.push(Spans::from(spans));
            }
            _ => {
                log::warn!("unhandled markdown event {:?}", event);
            }
        }
    }

    Text::from(lines)
}
