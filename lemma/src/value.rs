//! Values in Lemma
//! A value is the result of evaluating an [Form](crate::Form)

use crate::{form, Env, Form, Result, SymbolId};

/// A value from evaluating a [Form](crate::Form).
///
/// # Difference between [Form](crate::Form) and [Value]
/// All forms can be values, but not all values are forms due to function
/// bindings, special forms, and macro expansions.
/// [Value] is not serializable, but [Form](crate::Form) is.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Form value
    Form(Form),
    /// Callable function value
    Lambda(Lambda),
    /// Callable special form
    SpecialForm(SpecialForm),
}

/// A function as a value
#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    pub params: Vec<SymbolId>,
    pub body: Vec<Form>,
}

/// A function that evaluates special forms
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialForm {
    pub name: String,
    pub func: fn(&[Form], &mut Env) -> Result<Value>,
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Form(Form::Bool(value))
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Form(Form::Int(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Form(Form::string(value))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Form(Form::String(value))
    }
}

impl From<form::Form> for Value {
    fn from(value: form::Form) -> Self {
        Self::Form(value)
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Form(form) => write!(f, "{}", form),
            Value::Lambda(lambda) => write!(
                f,
                "<lambda ({})>",
                lambda
                    .params
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
    fn bool_to_string() {
        assert_eq!(Value::from(true).to_string(), "true");
        assert_eq!(Value::from(false).to_string(), "false");
    }

    #[test]
    fn int_to_string() {
        assert_eq!(Value::from(32).to_string(), "32");
    }

    #[test]
    fn string_to_string() {
        assert_eq!(Value::from("Hello world").to_string(), "\"Hello world\"");
    }

    #[test]
    fn keyword_to_string() {
        assert_eq!(
            Value::from(Form::keyword("a_keyword")).to_string(),
            ":a_keyword"
        );
    }

    #[test]
    fn form_to_string() {
        assert_eq!(Value::Form(Form::symbol("add")).to_string(), "add");

        assert_eq!(Value::Form(Form::keyword("add")).to_string(), ":add");

        assert_eq!(
            Value::Form(Form::List(vec![
                Form::symbol("add"),
                Form::Int(10),
                Form::string("ten"),
            ]))
            .to_string(),
            "(add 10 \"ten\")"
        )
    }
}
