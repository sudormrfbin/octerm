use std::{
    borrow::Cow,
    ops::{ControlFlow, Deref},
};

use meow::{
    components::{
        container::{Container, DimensionLen},
        line::Line,
        scroll::Scrollable,
        text::{Span, Spans, Text},
        Layout, Renderable, SizeArgs,
    },
    style::{Color, Style, Stylize},
};
use pulldown_cmark::{Event, Options, Tag};

pub struct Markdown<'t> {
    components: Layout<'t>,
}

impl Default for Markdown<'static> {
    fn default() -> Self {
        Markdown {
            components: Layout::vertical(),
        }
    }
}

impl<'t> Scrollable for Markdown<'t> {
    fn scroll_up(&mut self) {
        self.components.scroll_up();
    }

    fn scroll_down(&mut self) {
        self.components.scroll_down();
    }
}

impl<'t> Markdown<'t> {
    pub fn new(source: Cow<'t, str>) -> Self {
        let parser = pulldown_cmark::Parser::new_ext(&source, Options::ENABLE_STRIKETHROUGH);
        Self {
            // We set scrollable here unconditionally so that the Column
            // doesn't try to layout all the components: resizing to fit,
            // truncating them, etc. Also has the benifit of easily enabling
            // scrolling by wrapping in a Scroll.
            components: parse(&mut parser.into_iter(), None).scrollable(true),
        }
    }
}

impl<'t> Renderable for Markdown<'t> {
    fn render(&self, surface: &mut meow::Surface) {
        self.components.render(surface)
    }

    fn size(&self, args: SizeArgs) -> (meow::components::Width, meow::components::Height) {
        self.components.size(args)
    }
}

pub fn parse<'a, I: Iterator<Item = Event<'a>>>(
    events: &mut I,
    mut transform: Option<Box<dyn FnMut(&Event<'a>) -> ControlFlow<(), Option<Event<'a>>>>>,
) -> Layout<'static> {
    let mut tags: Vec<Tag> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut lines: Vec<Spans> = Vec::new();
    let mut column = Layout::vertical();

    let bold_style = Style::default().bold(true).fg(Color::Purple);
    let italic_style = Style::default().italic(true).fg(Color::Magenta);
    let code_style = Style::default().reverse(true);
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

    let flush_lines = |column: &mut Layout<'_>, lines: &mut Vec<Spans>| {
        column.push(Text::new(std::mem::take(lines)).cloned());
    };

    while let Some(mut event) = events.next() {
        if let Some(transform) = transform.as_mut() {
            match transform(&event) {
                ControlFlow::Break(_) => break,
                ControlFlow::Continue(Some(ev)) => event = ev,
                ControlFlow::Continue(None) => {}
            }
        };
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
                    Tag::BlockQuote => {
                        let quoted = parse(
                            events,
                            Some(Box::new(|ev| match ev {
                                Event::End(Tag::BlockQuote) => ControlFlow::Break(()),
                                _ => ControlFlow::Continue(None),
                            })),
                        );
                        flush_lines(&mut column, &mut lines);
                        // FIXME: Blockquotes have a final newline since a
                        // heading/paragraph/codeblock/list at the end automatically pushes
                        // a new line, see Event::End(..) below.
                        column.push(Container::new(BlockQuote::new(quoted)).fg(Color::Gray));
                        lines.push(Spans::default());
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
                        let code = Text::from(text.deref()).cloned();
                        flush_lines(&mut column, &mut lines);
                        column.push(
                            Container::new(code)
                                .bg(Color::Gray)
                                .width(DimensionLen::Max),
                        );
                    }
                    Some(Tag::Strikethrough) => spans.push(Span::new(text).strikethrough(true)),
                    Some(_) | None => spans.push(Span::new(text)),
                }
            }
            Event::Code(text) => {
                spans.push(Span::new("▐"));
                spans.push(Span::new(text).style(code_style.clone()));
                spans.push(Span::new("▌"));
            }
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
                flush_lines(&mut column, &mut lines);
                column.push(Line::horizontal());
                column.push(Text::from("\n"));
            }
            _ => {
                log::warn!("unhandled markdown event {:?}", event);
            }
        }
    }

    if !lines.is_empty() {
        column.push(Text::new(lines).cloned());
    }

    column
}

struct BlockQuote<R: Renderable> {
    pub child: R,
}

impl<R: Renderable> BlockQuote<R> {
    fn new(child: R) -> Self {
        Self { child }
    }
}

impl<R: Renderable> Renderable for BlockQuote<R> {
    fn render(&self, surface: &mut meow::Surface) {
        Layout::horizontal()
            .push(Line::vertical())
            .push(Line::vertical().blank())
            .push(&self.child)
            .render(surface);
    }

    fn size(&self, args: SizeArgs) -> (meow::components::Width, meow::components::Height) {
        let (width, height) = self.child.size(args);
        (width.saturating_add(2), height)
    }
}
