//! Types in Lisp virtual machine
use crate::codegen::Inst;
use crate::{Env, Error, Result};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};

/// All values in VM that can be manipulated
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Nil,
    Bool(bool),
    Int(i32),
    String(String),
    Symbol(SymbolId),
    Keyword(KeywordId),
    List(Vec<Val>),
    Bytecode(Vec<Inst>),
    Lambda(Lambda),
    NativeFn(NativeFn),
}

/// Forms are parsed expression that can be evaluated or be converted to [Val]
/// A serializable subset of [Val]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Form {
    Nil,
    Bool(bool),
    Int(i32),
    String(String),
    Symbol(SymbolId),
    Keyword(KeywordId),
    List(Vec<Form>),
}

/// A function object that closes over environment it was created in
#[derive(Debug, Clone)]
pub struct Lambda {
    pub params: Vec<SymbolId>,
    pub code: Vec<Inst>,
    pub env: Rc<RefCell<Env>>,
}

/// A native founction bound to given symbol
#[derive(Debug, Clone, PartialEq)]
pub struct NativeFn {
    pub symbol: SymbolId,
    pub func: fn(&[Val]) -> Result<Val>,
}

/// Identifier for Symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(String);

/// Identifier for Keywords
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeywordId(String);

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

impl Val {
    /// Shorhand for constructing [Val::String]
    pub fn string(s: &str) -> Self {
        Self::String(String::from(s))
    }

    /// Shorthand for constructing [Val::Symbol]
    pub fn symbol(id: &str) -> Self {
        Self::Symbol(SymbolId::from(id))
    }

    /// Shorthand for creating [Val::Keyword]
    pub fn keyword(id: &str) -> Self {
        Self::Keyword(KeywordId::from(id))
    }
}

impl SymbolId {
    /// Returns inner ID as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Lambda {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params && self.code == other.code && Rc::ptr_eq(&self.env, &other.env)
    }
}

impl std::fmt::Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::Nil => write!(f, "nil"),
            Val::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Val::Int(i) => write!(f, "{}", i),
            Val::String(s) => write!(f, "\"{}\"", s),
            Val::Keyword(k) => write!(f, "{}", k),
            Val::Symbol(s) => write!(f, "{}", s),
            Val::List(l) => match &l[..] {
                [quote, form] if quote == &Val::symbol("quote") => {
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
            Val::Lambda(l) => write!(
                f,
                "<lambda ({})>",
                l.params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Val::NativeFn(s) => write!(f, "<nativefn {}>", s.symbol),
            Val::Bytecode(_) => write!(f, "<bytecode>"),
        }
    }
}

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
                [quote, form] if quote == &Form::Symbol(SymbolId::from("quote")) => {
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

impl From<Form> for Val {
    fn from(value: Form) -> Self {
        match value {
            Form::Nil => Val::Nil,
            Form::Bool(b) => Val::Bool(b),
            Form::Int(i) => Val::Int(i),
            Form::String(s) => Val::String(s),
            Form::Symbol(s) => Val::Symbol(s),
            Form::Keyword(k) => Val::Keyword(k),
            Form::List(l) => Val::List(l.into_iter().map(|e| e.into()).collect()),
        }
    }
}

impl TryFrom<Val> for Form {
    type Error = Error;

    fn try_from(value: Val) -> std::result::Result<Self, Self::Error> {
        match value {
            Val::Nil => Ok(Form::Nil),
            Val::Bool(b) => Ok(Form::Bool(b)),
            Val::Int(i) => Ok(Form::Int(i)),
            Val::String(s) => Ok(Form::String(s)),
            Val::Symbol(s) => Ok(Form::Symbol(s)),
            Val::Keyword(k) => Ok(Form::Keyword(k)),
            Val::List(l) => Ok(Form::List(
                l.into_iter()
                    .map(|e| e.try_into())
                    .collect::<Result<Vec<_>>>()?,
            )),
            Val::Bytecode(_) => Err(Error::InvalidFormToExpr(
                "bytecode are not exprs".to_string(),
            )),
            Val::Lambda(_) => Err(Error::InvalidFormToExpr(
                "lambdas are not exprs".to_string(),
            )),
            Val::NativeFn(_) => Err(Error::InvalidFormToExpr(
                "nativefns are not exprs".to_string(),
            )),
        }
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
        assert_eq!(Val::Nil.to_string(), "nil");
    }

    #[test]
    fn bool_to_string() {
        assert_eq!(Val::Bool(true).to_string(), "true");
        assert_eq!(Val::Bool(false).to_string(), "false");
    }

    #[test]
    fn int_to_string() {
        assert_eq!(Val::Int(5).to_string(), "5");
        assert_eq!(Val::Int(0).to_string(), "0");
        assert_eq!(Val::Int(-99).to_string(), "-99");
    }

    #[test]
    fn string_to_string() {
        assert_eq!(Val::string("hello").to_string(), "\"hello\"");
        assert_eq!(
            Val::string("  hello  world  ").to_string(),
            "\"  hello  world  \"",
        );
    }

    #[test]
    fn symbol_to_string() {
        assert_eq!(Val::symbol("hello").to_string(), "hello");
    }

    #[test]
    fn keyword_to_string() {
        assert_eq!(Val::keyword("hello").to_string(), ":hello");
    }

    #[test]
    fn list_to_string() {
        assert_eq!(
            Val::List(vec![
                Val::symbol("my-func"),
                Val::Int(5),
                Val::string("string"),
            ])
            .to_string(),
            "(my-func 5 \"string\")"
        );
        assert_eq!(
            Val::List(vec![
                Val::symbol("hello"),
                Val::List(vec![
                    Val::symbol("world"),
                    Val::List(vec![Val::keyword("a_keyword"),])
                ]),
                Val::string("string"),
                Val::Int(10),
                Val::Int(-99),
            ])
            .to_string(),
            "(hello (world (:a_keyword)) \"string\" 10 -99)"
        );
    }

    #[test]
    fn quoted_to_string() {
        assert_eq!(
            Val::List(vec![Val::symbol("quote"), Val::symbol("hello")]).to_string(),
            "'hello"
        );
        assert_eq!(
            Val::List(vec![Val::symbol("quote"), Val::List(vec![])]).to_string(),
            "'()"
        );
        assert_eq!(
            Val::List(vec![
                Val::symbol("quote"),
                Val::List(vec![Val::Int(1), Val::Int(2), Val::Int(3),])
            ])
            .to_string(),
            "'(1 2 3)"
        );
        assert_eq!(
            Val::List(vec![
                Val::symbol("quote"),
                Val::List(vec![
                    Val::Int(1),
                    Val::Int(2),
                    Val::Int(3),
                    Val::List(vec![
                        Val::symbol("quote"),
                        Val::List(vec![Val::Int(4), Val::Int(5), Val::Int(6),])
                    ])
                ]),
            ])
            .to_string(),
            "'(1 2 3 '(4 5 6))"
        );
    }
}