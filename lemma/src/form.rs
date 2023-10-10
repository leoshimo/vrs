//! Forms in Lemma

use crate::{Env, Result};
use serde::{Deserialize, Serialize};

/// Forms that can be evaluated
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Form {
    Nil,
    Bool(bool),
    Int(i32),
    String(String),
    Symbol(SymbolId),
    Keyword(KeywordId),
    List(Vec<Form>),
    Lambda(Lambda),
    #[serde(skip)]
    NativeFunc(NativeFunc),
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// A function as a value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lambda {
    pub params: Vec<SymbolId>,
    pub body: Vec<Form>,
}

/// A function that evaluates special forms
#[derive(Debug, Clone, PartialEq)]
pub struct NativeFunc {
    pub symbol: SymbolId,
    pub func: fn(&[Form], &mut Env) -> Result<Form>,
}

impl Form {
    /// Shorhand for constructing [Form::String]
    pub fn string(s: &str) -> Self {
        Self::String(String::from(s))
    }

    /// Shorthand for constructing [Form::Symbol]
    pub fn symbol(id: &str) -> Self {
        Self::Symbol(SymbolId::from(id))
    }

    /// Shorthand for creating [Form::Keyword]
    pub fn keyword(id: &str) -> Self {
        Self::Keyword(KeywordId::from(id))
    }
}

/// Identifier for Symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(String);

impl SymbolId {
    /// Returns inner ID as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Identifier for Keyword
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeywordId(String);

impl std::fmt::Display for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Form::Nil => write!(f, "nil"),
            Form::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Form::Int(i) => write!(f, "{}", i),
            Form::String(s) => write!(f, "\"{}\"", s),
            Form::Keyword(k) => write!(f, "{}", k),
            Form::Symbol(s) => write!(f, "{}", s),
            Form::List(l) => match &l[..] {
                [quote, form] if quote == &Form::symbol("quote") => {
                    write!(f, "'{}", form)
                }
                _ => write!(
                    f,
                    "({})",
                    l.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            },
            Form::Lambda(l) => write!(
                f,
                "<lambda ({})>",
                l.params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Form::NativeFunc(s) => write!(f, "<nativefn {}>", s.symbol),
        }
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for KeywordId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ":{}", self.0)
    }
}

impl From<String> for SymbolId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SymbolId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for KeywordId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for KeywordId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nil_to_string() {
        assert_eq!(Form::Nil.to_string(), "nil");
    }

    #[test]
    fn bool_to_string() {
        assert_eq!(Form::Bool(true).to_string(), "true");
        assert_eq!(Form::Bool(false).to_string(), "false");
    }

    #[test]
    fn int_to_string() {
        assert_eq!(Form::Int(5).to_string(), "5");
        assert_eq!(Form::Int(0).to_string(), "0");
        assert_eq!(Form::Int(-99).to_string(), "-99");
    }

    #[test]
    fn string_to_string() {
        assert_eq!(Form::string("hello").to_string(), "\"hello\"");
        assert_eq!(
            Form::string("  hello  world  ").to_string(),
            "\"  hello  world  \"",
        );
    }

    #[test]
    fn symbol_to_string() {
        assert_eq!(Form::symbol("hello").to_string(), "hello");
    }

    #[test]
    fn keyword_to_string() {
        assert_eq!(Form::keyword("hello").to_string(), ":hello");
    }

    #[test]
    fn list_to_string() {
        assert_eq!(
            Form::List(vec![
                Form::symbol("my-func"),
                Form::Int(5),
                Form::string("string"),
            ])
            .to_string(),
            "(my-func 5 \"string\")"
        );
        assert_eq!(
            Form::List(vec![
                Form::symbol("hello"),
                Form::List(vec![
                    Form::symbol("world"),
                    Form::List(vec![Form::keyword("a_keyword"),])
                ]),
                Form::string("string"),
                Form::Int(10),
                Form::Int(-99),
            ])
            .to_string(),
            "(hello (world (:a_keyword)) \"string\" 10 -99)"
        );
    }

    #[test]
    fn quoted_to_string() {
        assert_eq!(
            Form::List(vec![Form::symbol("quote"), Form::symbol("hello")]).to_string(),
            "'hello"
        );
        assert_eq!(
            Form::List(vec![Form::symbol("quote"), Form::List(vec![])]).to_string(),
            "'()"
        );
        assert_eq!(
            Form::List(vec![
                Form::symbol("quote"),
                Form::List(vec![Form::Int(1), Form::Int(2), Form::Int(3),])
            ])
            .to_string(),
            "'(1 2 3)"
        );
        assert_eq!(
            Form::List(vec![
                Form::symbol("quote"),
                Form::List(vec![
                    Form::Int(1),
                    Form::Int(2),
                    Form::Int(3),
                    Form::List(vec![
                        Form::symbol("quote"),
                        Form::List(vec![Form::Int(4), Form::Int(5), Form::Int(6),])
                    ])
                ]),
            ])
            .to_string(),
            "'(1 2 3 '(4 5 6))"
        );
    }
}
