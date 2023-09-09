//! Forms in Lemma
//! A form is a object meant to be evaluated to yield a [Value]

use serde::{Deserialize, Serialize};

/// Forms that can be evaluated
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Form {
    Int(i32),
    String(String),
    Symbol(String),
    List(Vec<Form>),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_expressions() {
        assert_eq!(Form::Int(5).to_string(), "5");
        assert_eq!(Form::String(String::from("hello")).to_string(), "\"hello\"");
        assert_eq!(Form::Symbol(String::from("hello")).to_string(), "hello");
        assert_eq!(
            Form::List(vec![
                Form::Symbol(String::from("my-func")),
                Form::Int(5),
                Form::String(String::from("string")),
            ])
            .to_string(),
            "(my-func 5 \"string\")"
        );
    }
}
