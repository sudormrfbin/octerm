use std::fmt::Display;

use reedline::{
    default_emacs_keybindings, ColumnarMenu, DefaultPrompt, DefaultPromptSegment, Emacs, KeyCode,
    KeyModifiers, Prompt, Reedline, ReedlineEvent,
};

use crate::completion::completer;

pub fn line_editor() -> Reedline {
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
    // Set up the required keybindings
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    Reedline::create()
        .with_completer(Box::new(completer()))
        .with_edit_mode(edit_mode)
        .with_menu(reedline::ReedlineMenu::EngineCompleter(completion_menu))
}

pub fn prompt<T: Display>(p: T) -> impl Prompt {
    DefaultPrompt::new(
        DefaultPromptSegment::Basic(p.to_string()),
        DefaultPromptSegment::Empty,
    )
}
