//! Line editor for vrsctl REPL

use rustyline::{
    history::DefaultHistory,
    validate::{ValidationResult, Validator},
    Completer, Editor, Helper, Highlighter, Hinter, Result,
};

/// Create a line editor
pub fn editor() -> Result<Editor<ReplEditor, DefaultHistory>> {
    let editor = ReplEditor {};
    let mut rl = rustyline::Editor::new()?;
    rl.set_helper(Some(editor));
    Ok(rl)
}

/// Editor for vrsctl repl
#[derive(Completer, Helper, Highlighter, Hinter)]
pub struct ReplEditor {}

impl Validator for ReplEditor {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> Result<rustyline::validate::ValidationResult> {
        let mut stack = vec![];
        for c in ctx.input().chars() {
            match c {
                '(' => stack.push(c),
                ')' => match (stack.pop(), c) {
                    (Some('('), ')') => {}
                    (Some(wanted), _) => {
                        return Ok(ValidationResult::Invalid(Some(format!(
                            "{wanted} is not closed"
                        ))))
                    }
                    (None, c) => {
                        return Ok(ValidationResult::Invalid(Some(format!(
                            "{c} is not paired"
                        ))))
                    }
                },
                _ => {}
            }
        }
        if stack.is_empty() {
            Ok(ValidationResult::Valid(None))
        } else {
            Ok(ValidationResult::Incomplete)
        }
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}
