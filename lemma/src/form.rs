//! Forms in Lemma
//! A form is a object meant to be evaluated to yield a [Value]

use serde::{Deserialize, Serialize};

/// Forms that can be evaluated
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Form {
    Int(i32),
    String(String),
    Symbol(SymbolId),
    List(Vec<Form>),
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
}

/// Identifier for Symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(String);

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

impl std::fmt::Display for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Form::Int(i) => write!(f, "{}", i),
            Form::String(s) => write!(f, "\"{}\"", s),
            Form::Symbol(s) => write!(f, "{}", s),
            Form::List(l) => write!(
                f,
                "({})",
                l.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        }
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_expressions() {
        assert_eq!(Form::Int(5).to_string(), "5");
        assert_eq!(Form::string("hello").to_string(), "\"hello\"");
        assert_eq!(Form::symbol("hello").to_string(), "hello");
        assert_eq!(
            Form::List(vec![
                Form::symbol("my-func"),
                Form::Int(5),
                Form::string("string"),
            ])
            .to_string(),
            "(my-func 5 \"string\")"
        );
    }
}
