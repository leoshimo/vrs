//! Values in Lemma
//! A value is the result of evaluating an [Form](crate::Form)

use crate::{
    form::{self, KeywordId},
    Env, Form, Result, SymbolId,
};
use std::rc::Rc;

/// A value from evaluating a [Form](crate::Form).
///
/// # Difference between [Form](crate::Form) and [Value]
/// All forms can be values, but not all values are forms due to function
/// bindings, special forms, and macro expansions.
/// [Value] is not serializable, but [Form](crate::Form) is.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Integer value
    Int(i32),
    /// String value
    String(String),
    /// Keyword value
    Keyword(KeywordId),
    /// Form value
    Form(form::Form),
    /// Callable function value
    Func(Lambda),
    /// Callable special form
    SpecialForm(SpecialForm),
}

/// Parameters that function accepts
pub type Params = Vec<SymbolId>;

/// A function that evaluates function calls
#[derive(Clone)]
pub struct Lambda {
    pub params: Params,
    pub func: LambdaFn,
}

/// A function pointer stored in [Lambda]
pub type LambdaFn = Rc<dyn Fn(&Env) -> Result<Value>>;

impl PartialEq for Lambda {
    fn eq(&self, _other: &Self) -> bool {
        false // never equal
    }
}

impl std::fmt::Debug for Lambda {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<lambda ({})>",
            self.params
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

/// A function that evaluates special forms
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialForm {
    pub name: String,
    pub func: fn(&[Form], &Env) -> Result<Value>,
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<form::Form> for Value {
    fn from(value: form::Form) -> Self {
        match value {
            form::Form::Int(i) => Self::Int(i),
            form::Form::String(s) => Self::String(s),
            form::Form::Keyword(s) => Self::Keyword(s),
            _ => Self::Form(value),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{}", i),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Keyword(k) => write!(f, "{}", k),
            Value::Form(form) => write!(f, "{}", form),
            Value::Func(l) => write!(
                f,
                "<fn ({})>",
                l.params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Value::SpecialForm(s) => write!(f, "<spfn {}>", s.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_to_string() {
        assert_eq!(Value::Int(32).to_string(), "32");
    }

    #[test]
    fn string_to_string() {
        assert_eq!(
            Value::String(String::from("Hello world")).to_string(),
            "\"Hello world\""
        );
    }

    #[test]
    fn keyword_to_string() {
        assert_eq!(
            Value::Keyword(KeywordId::from("a_keyword")).to_string(),
            ":a_keyword"
        );
    }

    #[test]
    fn form_to_string() {
        assert_eq!(Value::Form(form::Form::symbol("add")).to_string(), "add");

        assert_eq!(Value::Form(form::Form::keyword("add")).to_string(), ":add");

        assert_eq!(
            Value::Form(form::Form::List(vec![
                Form::symbol("add"),
                Form::Int(10),
                Form::string("ten"),
            ]))
            .to_string(),
            "(add 10 \"ten\")"
        )
    }
}
