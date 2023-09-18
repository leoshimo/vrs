//! Lexer for Lemma
use std::iter::Peekable;

use crate::{Error, Result};

/// Parsed Tokens from String
#[derive(Debug, PartialEq)]
pub enum Token {
    Nil,
    Bool(bool),
    Int(i32),
    String(String),
    Symbol(String),
    Keyword(String),
    ParenLeft,
    ParenRight,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Nil => write!(f, "nil"),
            Token::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Token::Int(i) => write!(f, "{}", i),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Symbol(s) => write!(f, "{}", s),
            Token::Keyword(s) => write!(f, ":{}", s),
            Token::ParenLeft => write!(f, "("),
            Token::ParenRight => write!(f, ")"),
        }
    }
}

/// Tokenize entire expression as vector
pub(crate) fn lex(expr: &str) -> Result<Vec<Token>> {
    let tokens = Tokens::new(expr).collect::<Result<Vec<_>>>()?;
    Ok(tokens)
}

/// An iterator over Tokens
struct Tokens<'a> {
    inner: Peekable<std::str::Chars<'a>>,
}

impl Tokens<'_> {
    /// Create Tokens iterator from &str
    fn new(expr: &str) -> Tokens<'_> {
        Tokens {
            inner: expr.chars().peekable(),
        }
    }

    /// Parse next symbol from inner iterator
    fn next_symbol(&mut self) -> Result<Token> {
        let expr: String =
            std::iter::from_fn(|| self.inner.next_if(|ch| !is_symbol_delimiter(ch))).collect();
        match expr.as_str() {
            "true" => Ok(Token::Bool(true)),
            "false" => Ok(Token::Bool(false)),
            "nil" => Ok(Token::Nil),
            _ => Ok(Token::Symbol(expr)),
        }
    }

    /// Pares the next int
    fn next_int(&mut self) -> Result<Token> {
        let expr: String =
            std::iter::from_fn(|| self.inner.next_if(|ch| !is_symbol_delimiter(ch))).collect();
        let num = expr
            .parse::<i32>()
            .map_err(|_| Error::FailedToLex(format!("Unable to parse integer - {expr}")))?;
        Ok(Token::Int(num))
    }

    /// Parse next punctuation
    fn next_punct(&mut self) -> Result<Token> {
        let ch = self
            .inner
            .next()
            .ok_or(Error::FailedToLex("Expected punctuation".to_string()))?;
        match ch {
            '(' => Ok(Token::ParenLeft),
            ')' => Ok(Token::ParenRight),
            _ => Err(Error::FailedToLex(format!("Unexpected punctuation - {ch}"))),
        }
    }

    /// Parse next string
    fn next_string(&mut self) -> Result<Token> {
        let ch = self.inner.next().ok_or(Error::FailedToLex(
            "Expected opening string quotation".to_string(),
        ))?;
        if ch != '\"' {
            return Err(Error::FailedToLex(format!(
                "Expected opening string quotation - found {ch}"
            )));
        }

        let expr: String = std::iter::from_fn(|| self.inner.next_if(|ch| *ch != '\"')).collect();

        let ch = self.inner.next().ok_or(Error::FailedToLex(
            "Expected closing string quotation".to_string(),
        ))?;
        if ch != '\"' {
            return Err(Error::FailedToLex(format!(
                "Expected closing string quotation - found {ch}"
            )));
        }

        Ok(Token::String(expr))
    }

    /// Parse keyword
    fn next_keyword(&mut self) -> Result<Token> {
        let ch = self.inner.next().ok_or(Error::FailedToLex(
            "Expected symbol : for start of keyword".to_string(),
        ))?;
        if ch != ':' {
            return Err(Error::FailedToLex(format!(
                "Expected symbol : for keyword - found {}",
                ch
            )));
        }

        let keyword =
            std::iter::from_fn(|| self.inner.next_if(|ch| !is_symbol_delimiter(ch))).collect();

        Ok(Token::Keyword(keyword))
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
                }
                '\"' => self.next_string(),
                ':' => self.next_keyword(),
                _ if is_list_delimiter(ch) => self.next_punct(),
                _ if ch.is_numeric() || ch == &'-' => self.next_int(),
                _ => self.next_symbol(),
            };
            return Some(token);
        }
        None
    }
}

/// Return whether or not a given character is a symbol delimiter
fn is_symbol_delimiter(ch: &char) -> bool {
    ch.is_whitespace() || is_list_delimiter(ch)
}

/// Return whether or not given character is a list delimiter
fn is_list_delimiter(ch: &char) -> bool {
    ch == &'(' || ch == &')'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_nil() {
        assert_eq!(lex("nil"), Ok(vec![Token::Nil]));
    }

    #[test]
    fn lex_bool() {
        assert_eq!(lex("true"), Ok(vec![Token::Bool(true)]));
        assert_eq!(lex("false"), Ok(vec![Token::Bool(false)]));
    }

    #[test]
    fn lex_int() {
        assert_eq!(lex("1"), Ok(vec![Token::Int(1)]));
        assert_eq!(lex("     1     "), Ok(vec![Token::Int(1)]));
        assert_eq!(lex("-99"), Ok(vec![Token::Int(-99)]));
    }

    #[test]
    fn lex_symbol() {
        assert_eq!(lex("hello"), Ok(vec![Token::Symbol(String::from("hello"))]));
        assert_eq!(
            lex("hello world"),
            Ok(vec![
                Token::Symbol(String::from("hello")),
                Token::Symbol(String::from("world")),
            ])
        );
        assert_eq!(
            lex("hello_world"),
            Ok(vec![Token::Symbol(String::from("hello_world"))])
        );
        assert_eq!(
            lex("hello-world"),
            Ok(vec![Token::Symbol(String::from("hello-world"))])
        );
        assert_eq!(
            lex("    hello    "),
            Ok(vec![Token::Symbol(String::from("hello"))])
        );
    }

    #[test]
    fn lex_string() {
        assert_eq!(lex("\"\""), Ok(vec![Token::String("".to_string())]));

        assert_eq!(
            lex("\"hello\""),
            Ok(vec![Token::String("hello".to_string())])
        );
        assert_eq!(
            lex("      \"hello\"      "),
            Ok(vec![Token::String("hello".to_string())])
        );

        assert_eq!(
            lex("\"  hello  world\""),
            Ok(vec![Token::String("  hello  world".to_string())])
        );
        assert_eq!(
            lex("      \"hello  world  \"      "),
            Ok(vec![Token::String("hello  world  ".to_string())])
        );
    }

    #[test]
    fn lex_list() {
        assert_eq!(
            lex("(add 1 2 \"three\")"),
            Ok(vec![
                Token::ParenLeft,
                Token::Symbol(String::from("add")),
                Token::Int(1),
                Token::Int(2),
                Token::String("three".to_string()),
                Token::ParenRight
            ])
        );

        assert_eq!(
            lex("      (add       1      2 \"three\" )"),
            Ok(vec![
                Token::ParenLeft,
                Token::Symbol(String::from("add")),
                Token::Int(1),
                Token::Int(2),
                Token::String("three".to_string()),
                Token::ParenRight
            ])
        );

        assert_eq!(
            lex("(() ()     (( )) )"),
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
                Token::ParenRight,
            ])
        )
    }

    #[test]
    fn lex_keywords() {
        assert_eq!(
            lex(":a_keyword"),
            Ok(vec![Token::Keyword("a_keyword".to_string()),])
        );

        assert_eq!(
            lex("(:a_keyword)"),
            Ok(vec![
                Token::ParenLeft,
                Token::Keyword("a_keyword".to_string()),
                Token::ParenRight,
            ])
        );

        assert_eq!(
            lex("(a_func :a_keyword 3)"),
            Ok(vec![
                Token::ParenLeft,
                Token::Symbol("a_func".to_string()),
                Token::Keyword("a_keyword".to_string()),
                Token::Int(3),
                Token::ParenRight,
            ])
        );
    }

    #[test]
    fn lex_nested() {
        assert_eq!(
            lex("(defun hello (x y z) (print \"hello\"))"),
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
            ])
        );
    }
}
