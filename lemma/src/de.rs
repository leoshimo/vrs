//! Deserialization for Lemma [Form](crate::Form)
use crate::{parse::parse, Form, Result};

/// Deserialize a &str into a [Form](crate::Form)
pub fn from_str(s: &str) -> Result<Form> {
    // TODO: Implement serde traits? Piggybacks off lemma parser today.
    parse(s)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn str_to_form_int() {
        assert_eq!(from_str("10"), Ok(Form::Int(10)),);
        assert_eq!(from_str("0"), Ok(Form::Int(0)),);
        assert_eq!(from_str("-10"), Ok(Form::Int(-10)),);
    }

    #[test]
    fn str_to_form_string() {
        assert_eq!(from_str("\"Hello world\""), Ok(Form::string("Hello world")),);
    }

    #[test]
    fn str_to_form_symbol() {
        assert_eq!(from_str("kwd"), Ok(Form::symbol("kwd")),);
        assert_eq!(from_str("an_keyword"), Ok(Form::symbol("an_keyword")),);
    }

    #[test]
    fn str_to_form_keyword() {
        assert_eq!(from_str(":kwd"), Ok(Form::keyword("kwd")),);
        assert_eq!(from_str(":an_keyword"), Ok(Form::keyword("an_keyword")),);
    }

    #[test]
    fn str_to_form_list() {
        assert_eq!(
            from_str("(hello (world (:a_keyword)) \"string\" 10 -99)"),
            Ok(Form::List(vec![
                Form::symbol("hello"),
                Form::List(vec![
                    Form::symbol("world"),
                    Form::List(vec![Form::keyword("a_keyword"),])
                ]),
                Form::string("string"),
                Form::Int(10),
                Form::Int(-99),
            ]))
        )
    }
}
