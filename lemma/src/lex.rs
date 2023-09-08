//! Lexer for Lemma
use std::iter::Peekable;

use crate::{Error, Result};

/// Parsed Tokens from String
#[derive(Debug, PartialEq)]
pub enum Token {
    Int(i32),
    Float(f32),
    String(String),
    Symbol(String),
    ParenLeft,
    ParenRight,
}

/// An iterator over Tokens
struct Tokens<'a> {
    inner: Peekable<std::str::Chars<'a>>,
}

impl Tokens<'_> {
    /// Create Tokens iterator from &str
    fn new<'a>(expr: &'a str) -> Tokens<'a> {
        Tokens {
            inner: expr.chars().peekable(),
        }
    }

    /// Parse next symbol from inner iterator
    fn next_symbol(&mut self) -> Result<Token> {
        let expr = std::iter::from_fn(|| self.inner.next_if(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())).collect();
        Ok(Token::Symbol(expr))
    }

    /// Pares the next int
    fn next_int(&mut self) -> Result<Token> {
        let expr: String =
            std::iter::from_fn(|| self.inner.next_if(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())).collect();
        let num = expr
            .parse::<i32>()
            .map_err(|_| Error::FailedToLex(format!("Unable to parse integer - {expr}")))?;
        Ok(Token::Int(num))
    }

    /// Parse next punctuation
    fn next_punct(&mut self) -> Result<Token> {
        let ch = self.inner.next().ok_or(Error::FailedToLex(format!("Expected punctuation")))?;
        match ch {
            '(' => Ok(Token::ParenLeft),
            ')' => Ok(Token::ParenRight),
            _ => Err(Error::FailedToLex(format!("Unexpected punctuation - {ch}")))
        }
    }

    /// Parse next string
    fn next_string(&mut self) -> Result<Token> {
        let ch = self.inner.next().ok_or(Error::FailedToLex("Expected opening string quotation".to_string()))?;
        if ch != '\"' {
            return Err(Error::FailedToLex(format!("Expected opening string quotation - found {ch}")));
        }

        let expr: String = std::iter::from_fn(|| self.inner.next_if(|ch| *ch != '\"')).collect();

        let ch = self.inner.next().ok_or(Error::FailedToLex("Expected closing string quotation".to_string()))?;
        if ch != '\"' {
            return Err(Error::FailedToLex(format!("Expected closing string quotation - found {ch}")));
        }

        Ok(Token::String(expr))
    }
}

impl<'a> Iterator for Tokens<'a> {


    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(ch) = self.inner.peek() {
            let token = match ch {
                _ if ch.is_whitespace() => {
                    let _ = self.inner.next();
                    continue;
                },
                '\"' => self.next_string(),
                _ if ch.is_ascii_punctuation() => self.next_punct(),
                _ if ch.is_numeric() => self.next_int(),
                _ => self.next_symbol(),
            };
            return Some(token);
        }
        None
    }
}

/// Lex an expression into a list of tokens
pub fn lex(expr: &str) -> Result<Vec<Token>> {
    let tokens = Tokens::new(expr).collect::<Result<Vec<_>>>()?;
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_int() {
        assert_eq!(lex("1"), Ok(vec![Token::Int(1)]));
        assert_eq!(lex("     1     "), Ok(vec![Token::Int(1)]));

    }

    #[test]
    fn lex_symbol() {
        assert_eq!(lex("hello"), Ok(vec![Token::Symbol(String::from("hello"))]));
        assert_eq!(
            lex("    hello    "),
            Ok(vec![Token::Symbol(String::from("hello"))])
        );
    }

    #[test]
    fn lex_string() {
        assert_eq!(lex("\"\""), Ok(vec![Token::String("".to_string())]));

        assert_eq!(lex("\"hello\""), Ok(vec![Token::String("hello".to_string())]));
        assert_eq!(lex("      \"hello\"      "), Ok(vec![Token::String("hello".to_string())]));

        assert_eq!(lex("\"  hello  world\""), Ok(vec![Token::String("  hello  world".to_string())]));
        assert_eq!(lex("      \"hello  world  \"      "), Ok(vec![Token::String("hello  world  ".to_string())]));
    }

    #[test]
    fn lex_list() {
        assert_eq!(lex("(add 1 2 \"three\")"), Ok(vec![
            Token::ParenLeft,
            Token::Symbol(String::from("add")),
            Token::Int(1),
            Token::Int(2),
            Token::String("three".to_string()),
            Token::ParenRight]));

        assert_eq!(lex("      (add       1      2 \"three\" )"), Ok(vec![
            Token::ParenLeft,
            Token::Symbol(String::from("add")),
            Token::Int(1),
            Token::Int(2),
            Token::String("three".to_string()),
            Token::ParenRight]));

        assert_eq!(lex("(() ()     (( )) )"),
                   Ok(vec![
                       Token::ParenLeft,
                       Token::ParenLeft,
                       Token::ParenRight,
                       Token::ParenLeft,
                       Token::ParenRight,

                       Token::ParenLeft,
                       Token::ParenLeft,
                       Token::ParenRight,
                       Token::ParenRight,
                       Token::ParenRight]))
    }

    #[test]
    fn lex_nested() {
        assert_eq!(lex("(defun hello (x y z) (print \"hello\"))"),
                       Ok(vec![
                           Token::ParenLeft,
                           Token::Symbol("defun".to_string()),
                           Token::Symbol("hello".to_string()),
                           Token::ParenLeft,
                           Token::Symbol("x".to_string()),
                           Token::Symbol("y".to_string()),
                           Token::Symbol("z".to_string()),
                           Token::ParenRight,
                           Token::ParenLeft,
                           Token::Symbol("print".to_string()),
                           Token::String("hello".to_string()),
                           Token::ParenRight,
                           Token::ParenRight,
                       ]));
    }
}
