//! Serialization for Lemma [Form](crate::Form)
use crate::{Form, Result};

/// Serialize a [Form](crate::Form) into a string
pub fn to_string(v: Form) -> Result<String> {
    // TODO: Implement serde traits?
    Ok(v.to_string())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn form_int_to_string() {
        assert_eq!(to_string(Form::Int(0)), Ok(String::from("0")),);
        assert_eq!(to_string(Form::Int(10)), Ok(String::from("10")),);
        assert_eq!(to_string(Form::Int(-99)), Ok(String::from("-99")),);
    }

    #[test]
    fn form_string_to_string() {
        assert_eq!(
            to_string(Form::string("hello")),
            Ok(String::from("\"hello\"")),
        );
        assert_eq!(
            to_string(Form::string("  hello  world  ")),
            Ok(String::from("\"  hello  world  \""))
        );
    }

    #[test]
    fn form_symbol_to_string() {
        assert_eq!(
            to_string(Form::symbol("a_symbol")),
            Ok(String::from("a_symbol"))
        );
    }

    #[test]
    fn form_keyword_to_string() {
        assert_eq!(
            to_string(Form::keyword("a_keyword")),
            Ok(String::from(":a_keyword"))
        );
    }

    #[test]
    fn form_list_to_string() {
        assert_eq!(
            to_string(Form::List(vec![
                Form::symbol("hello"),
                Form::List(vec![
                    Form::symbol("world"),
                    Form::List(vec![Form::keyword("a_keyword"),])
                ]),
                Form::string("string"),
                Form::Int(10),
                Form::Int(-99),
            ])),
            Ok(String::from(
                "(hello (world (:a_keyword)) \"string\" 10 -99)"
            )),
        )
    }
}
