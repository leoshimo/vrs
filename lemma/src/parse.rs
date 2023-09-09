//! Parser for Lemma
use crate::lex::{lex, Token};
use crate::Form;
use crate::{Error, Result};

use std::iter::Peekable;

/// Parse a given expression as form
pub fn parse(expr: &str) -> Result<Form> {
    let mut tokens = lex(expr)?.into_iter().peekable();
    let form = parse_form(&mut tokens)?;
    if tokens.peek().is_some() {
        return Err(Error::FailedToParse("Unterminated expression".to_string()));
    }
    Ok(form)
}

/// Parse single expression into a form. Returns result of tuple of parsed form and remaining tokens
fn parse_form<I>(tokens: &mut Peekable<I>) -> Result<Form>
where
    I: Iterator<Item = Token>,
{
    let next = tokens.next().ok_or(Error::EmptyExpression)?;
    let form = match next {
        Token::Int(i) => Form::Int(i),
        Token::Symbol(s) => Form::Symbol(s),
        Token::String(s) => Form::String(s),
        Token::ParenLeft => {
            let mut items = vec![];
            while let Some(next) = tokens.peek() {
                if next == &Token::ParenRight {
                    break;
                }
                items.push(parse_form(tokens)?);
            }
            if tokens.peek() != Some(&Token::ParenRight) {
                return Err(Error::FailedToParse(
                    "Expected closing parenthesis".to_string(),
                ));
            }
            tokens.next(); // discard ParenRight
            Form::List(items)
        }
        _ => {
            return Err(Error::FailedToParse(
                "Unexpected token while parsing expression".to_string(),
            ))
        }
    };
    Ok(form)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        assert_eq!(parse(""), Err(Error::EmptyExpression));
        assert_eq!(parse("            "), Err(Error::EmptyExpression));
    }

    #[test]
    fn parse_int() {
        assert_eq!(parse("1"), Ok(Form::Int(1)));
        assert_eq!(parse("     1     "), Ok(Form::Int(1)));
    }

    #[test]
    fn parse_symbol() {
        assert_eq!(parse("hello"), Ok(Form::Symbol(String::from("hello"))));
        assert_eq!(
            parse("    hello    "),
            Ok(Form::Symbol(String::from("hello")))
        );
    }

    #[test]
    fn parse_string() {
        assert_eq!(parse("\"\""), Ok(Form::String("".to_string())));

        assert_eq!(parse("\"hello\""), Ok(Form::String("hello".to_string())));
        assert_eq!(
            parse("      \"hello\"      "),
            Ok(Form::String("hello".to_string()))
        );

        assert_eq!(
            parse("\"  hello  world\""),
            Ok(Form::String("  hello  world".to_string()))
        );
        assert_eq!(
            parse("      \"hello  world  \"      "),
            Ok(Form::String("hello  world  ".to_string()))
        );
    }

    #[test]
    fn parse_list() {
        assert_eq!(
            parse("(add 1 2 \"three\")"),
            Ok(Form::List(vec![
                Form::Symbol("add".to_string()),
                Form::Int(1),
                Form::Int(2),
                Form::String("three".to_string()),
            ]))
        );

        assert_eq!(
            parse("      (add       1      2 \"three\" )"),
            Ok(Form::List(vec![
                Form::Symbol("add".to_string()),
                Form::Int(1),
                Form::Int(2),
                Form::String("three".to_string()),
            ]))
        );

        assert_eq!(
            parse("(() ()     (( )) )"),
            Ok(Form::List(vec![
                Form::List(vec![]),
                Form::List(vec![]),
                Form::List(vec![Form::List(vec![]),]),
            ]))
        )
    }

    #[test]
    fn parse_nested() {
        assert_eq!(
            parse("(defun hello (x y z) (print \"hello\"))"),
            Ok(Form::List(vec![
                Form::Symbol("defun".to_string()),
                Form::Symbol("hello".to_string()),
                Form::List(vec![
                    Form::Symbol("x".to_string()),
                    Form::Symbol("y".to_string()),
                    Form::Symbol("z".to_string()),
                ]),
                Form::List(vec![
                    Form::Symbol("print".to_string()),
                    Form::String("hello".to_string()),
                ]),
            ]),)
        );
    }

    #[test]
    fn parse_partial_form() {
        assert!(
            matches!(parse("1 2 3"), Err(Error::FailedToParse(_))),
            "parse should fail if entire expression cannot be consumed as single form"
        );
    }

    #[test]
    fn parse_unterminated_list() {
        assert!(matches!(parse("(1 2 3"), Err(Error::FailedToParse(_))));
    }
}
