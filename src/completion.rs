use reedline::{Completer, DefaultCompleter};

use crate::parser::types::{Adapter, Command, Consumer, Producer};

pub fn completer() -> impl Completer {
    let completions = Command::all()
        .iter()
        .chain(&Producer::all())
        .chain(&Adapter::all())
        .chain(&Consumer::all())
        .map(ToString::to_string)
        .collect();
    let mut completer = DefaultCompleter::default().set_min_word_len(0);
    // Calling set_min_word_len after DefaultCompleter::new(completions)
    // has no effect since minimum length has to be set first.
    completer.insert(completions);
    completer
}

struct ReplCompleter;

impl Completer for ReplCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<reedline::Suggestion> {
        let suggestions = Vec::new();
        let line = &line[..pos];

        if line.find('|').is_some() {}

        suggestions
    }
}
