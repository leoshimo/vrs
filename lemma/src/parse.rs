//! Parser for Lemma
use crate::lex::{lex, Token};
use crate::Expr;
use crate::{Error, Result};

use std::iter::Peekable;

/// Parse a given string as expression
pub fn parse(expr: &str) -> Result<Expr> {
    let mut tokens = lex(expr)?.into_iter().peekable();
    let expr = parse_expr(&mut tokens)?;
    if tokens.peek().is_some() {
        return Err(Error::FailedToParse(format!("Unterminated expression")));
    }
    Ok(expr)
}

/// Parse single expression. Returns result of tuple of parsed expression and remaining tokens
fn parse_expr<I>(tokens: &mut Peekable<I>) -> Result<Expr>
where
    I: Iterator<Item = Token>,
{
    let next = tokens.next().ok_or(Error::EmptyExpression)?;
    let expr = match next {
        Token::Int(i) => Expr::Int(i),
        Token::Symbol(s) => Expr::Symbol(s),
        Token::String(s) => Expr::String(s),
        Token::ParenLeft => {
            let mut items = vec![];
            while let Some(next) = tokens.peek() {
                if next == &Token::ParenRight {
                    break;
                }
                items.push(parse_expr(tokens)?);
            }
            if tokens.peek() != Some(&Token::ParenRight) {
                return Err(Error::FailedToParse(
                    "Expected closing parenthesis".to_string(),
                ));
            }
            tokens.next(); // discard ParenRight
            Expr::List(items)
        }
        _ => {
            return Err(Error::FailedToParse(
                "Unexpected token while parsing expression".to_string(),
            ))
        }
    };
    Ok(expr)
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
        assert_eq!(parse("1"), Ok(Expr::Int(1)));
        assert_eq!(parse("     1     "), Ok(Expr::Int(1)));
    }

    #[test]
    fn parse_symbol() {
        assert_eq!(parse("hello"), Ok(Expr::Symbol(String::from("hello"))));
        assert_eq!(
            parse("    hello    "),
            Ok(Expr::Symbol(String::from("hello")))
        );
    }

    #[test]
    fn parse_string() {
        assert_eq!(parse("\"\""), Ok(Expr::String("".to_string())));

        assert_eq!(parse("\"hello\""), Ok(Expr::String("hello".to_string())));
        assert_eq!(
            parse("      \"hello\"      "),
            Ok(Expr::String("hello".to_string()))
        );

        assert_eq!(
            parse("\"  hello  world\""),
            Ok(Expr::String("  hello  world".to_string()))
        );
        assert_eq!(
            parse("      \"hello  world  \"      "),
            Ok(Expr::String("hello  world  ".to_string()))
        );
    }

    #[test]
    fn parse_list() {
        assert_eq!(
            parse("(add 1 2 \"three\")"),
            Ok(Expr::List(vec![
                Expr::Symbol("add".to_string()),
                Expr::Int(1),
                Expr::Int(2),
                Expr::String("three".to_string()),
            ]))
        );

        assert_eq!(
            parse("      (add       1      2 \"three\" )"),
            Ok(Expr::List(vec![
                Expr::Symbol("add".to_string()),
                Expr::Int(1),
                Expr::Int(2),
                Expr::String("three".to_string()),
            ]))
        );

        assert_eq!(
            parse("(() ()     (( )) )"),
            Ok(Expr::List(vec![
                Expr::List(vec![]),
                Expr::List(vec![]),
                Expr::List(vec![Expr::List(vec![]),]),
            ]))
        )
    }

    #[test]
    fn parse_nested() {
        assert_eq!(
            parse("(defun hello (x y z) (print \"hello\"))"),
            Ok(Expr::List(vec![
                Expr::Symbol("defun".to_string()),
                Expr::Symbol("hello".to_string()),
                Expr::List(vec![
                    Expr::Symbol("x".to_string()),
                    Expr::Symbol("y".to_string()),
                    Expr::Symbol("z".to_string()),
                ]),
                Expr::List(vec![
                    Expr::Symbol("print".to_string()),
                    Expr::String("hello".to_string()),
                ]),
            ]),)
        );
    }

    #[test]
    fn parse_partial_form() {
        assert!(
            matches!(parse("1 2 3"), Err(Error::FailedToParse(_))),
            "parse returns expression for full expression forms"
        );
    }

    #[test]
    fn parse_unterminated_list() {
        assert!(matches!(parse("(1 2 3"), Err(Error::FailedToParse(_))));
    }
}
