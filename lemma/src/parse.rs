//! Parser for Lemma
use crate::lex::{Token, Tokens};
use crate::Expr;
use crate::{Error, Result};

type PeekableTokens<'a> = std::iter::Peekable<Tokens<'a>>;

/// Parse a given string as expression
pub fn parse(expr: &str) -> Result<Expr> {
    let mut tokens = Tokens::new(expr).peekable();
    let expr = parse_expr(&mut tokens)?;
    if tokens.next().is_some() {
        todo!("Handle unterminated expressions");
    }
    Ok(expr)
}

/// Parse single expression
fn parse_expr(tokens: &mut PeekableTokens<'_>) -> Result<Expr> {
    let next = tokens.next().ok_or(Error::EmptyExpression)??;
    match next {
        Token::Int(i) => Ok(Expr::Int(i)),
        Token::Symbol(s) => Ok(Expr::Symbol(s)),
        Token::String(s) => Ok(Expr::String(s)),
        _ => Err(Error::FailedToParse(
            "Unexpected token while parsing expression".to_string(),
        )),
    }
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
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string()),
                    Expr::Symbol("c".to_string()),
                ]),
                Expr::List(vec![
                    Expr::Symbol("print".to_string()),
                    Expr::String("hello".to_string()),
                ]),
            ]),)
        );
    }
}
