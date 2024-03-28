//! Parser for Lyric
use crate::lex::{lex, Token};
use crate::types::KeywordId;
use crate::{Error, Result};
use crate::{Form, SymbolId};

use std::iter::Peekable;

/// Parse a given expression as form
pub fn parse(expr: &str) -> Result<Form> {
    let mut tokens = lex(expr)?.into_iter().peekable();
    let form = parse_form(&mut tokens)?;
    if tokens.peek().is_some() {
        return Err(Error::IncompleteExpression(
            "Unable to parse full expression - unbalanced trailing expressions".to_string(),
        ));
    }
    Ok(form)
}

/// Parse a script containing zero or more top-level forms.
pub fn parse_script(expr: &str) -> Result<Vec<Form>> {
    let mut tokens = lex(expr)?.into_iter().peekable();
    let mut forms = Vec::new();
    while tokens.peek().is_some() {
        forms.push(parse_form(&mut tokens)?);
    }
    Ok(forms)
}

/// Parse a source unit while retaining exact call locations. Symbol identity,
/// quotation, and Form serialization remain unchanged.
pub fn parse_source(text: &str, file: &str, line: usize, column: usize) -> Result<Vec<Form>> {
    let mut forms = parse_script(text)?;
    let tokens = crate::lex::lex_spanned(text)?;
    let mut index = 0;
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    fn locate(
        form: &mut Form,
        tokens: &[(Token, usize, usize)],
        index: &mut usize,
        text: &str,
        file: &str,
        line: usize,
        column: usize,
        starts: &[usize],
    ) {
        let start = tokens[*index].1;
        match &tokens[*index].0 {
            Token::ParenLeft => {
                *index += 1;
                if let Form::List(items) = form {
                    for item in items {
                        locate(item, tokens, index, text, file, line, column, starts);
                    }
                }
                *index += 1;
            }
            Token::Quote | Token::Quasiquote | Token::Unquote | Token::UnquoteSplicing => {
                *index += 1;
                if let Form::List(items) = form {
                    locate(
                        &mut items[1],
                        tokens,
                        index,
                        text,
                        file,
                        line,
                        column,
                        starts,
                    );
                }
            }
            _ => *index += 1,
        }
        let end = tokens[*index - 1].2;
        if matches!(form, Form::List(_)) {
            let row = starts.partition_point(|offset| *offset <= start) - 1;
            let col = text[starts[row]..start].chars().count() + if row == 0 { column } else { 1 };
            let site = crate::source::SourceSite {
                file: file.into(),
                line: line + row,
                column: col,
                expression: text[start..end].into(),
                form: form.to_string(),
                generated: false,
            };
            crate::source::attach(form, site);
        }
    }
    for form in &mut forms {
        locate(form, &tokens, &mut index, text, file, line, column, &starts);
    }
    Ok(forms)
}

/// Parse single expression into a form. Returns result of tuple of parsed form and remaining tokens
fn parse_form<I>(tokens: &mut Peekable<I>) -> Result<Form>
where
    I: Iterator<Item = Token>,
{
    parse_form_at_depth(tokens, 0)
}

fn parse_form_at_depth<I>(tokens: &mut Peekable<I>, depth: usize) -> Result<Form>
where
    I: Iterator<Item = Token>,
{
    if depth > 256 {
        return Err(Error::InvalidExpression(
            "source nesting exceeds 256 levels".into(),
        ));
    }
    let next = tokens
        .next()
        .ok_or(Error::IncompleteExpression("Expected a form".to_string()))?;
    let form = match next {
        Token::Nil => Form::Nil,
        Token::Bool(b) => Form::Bool(b),
        Token::Int(i) => Form::Int(i),
        Token::Symbol(s) => Form::Symbol(SymbolId::from(s)),
        Token::String(s) => Form::String(s),
        Token::Keyword(k) => Form::Keyword(KeywordId::from(k)),
        Token::ParenLeft => {
            let mut items = vec![];
            while let Some(next) = tokens.peek() {
                if next == &Token::ParenRight {
                    break;
                }
                items.push(parse_form_at_depth(tokens, depth + 1)?);
            }
            if tokens.peek() != Some(&Token::ParenRight) {
                return Err(Error::IncompleteExpression(
                    "Expected closing parenthesis".to_string(),
                ));
            }
            tokens.next(); // discard ParenRight
            Form::List(items)
        }
        Token::ParenRight => {
            return Err(Error::IncompleteExpression(
                "Unexpected closing parenthesis while parsing expression".to_string(),
            ))
        }
        prefix @ (Token::Quote | Token::Quasiquote | Token::Unquote | Token::UnquoteSplicing) => {
            let name = match prefix {
                Token::Quote => "quote",
                Token::Quasiquote => "quasiquote",
                Token::Unquote => "unquote",
                Token::UnquoteSplicing => "unquote-splicing",
                _ => unreachable!(),
            };
            match tokens.peek() {
                None => {
                    return Err(Error::IncompleteExpression(format!(
                        "Expected a form after {prefix}"
                    )))
                }
                Some(Token::ParenRight) => {
                    return Err(Error::InvalidExpression(format!(
                        "Expected a form after {prefix}, found )"
                    )))
                }
                _ => (),
            }
            let quoted = parse_form_at_depth(tokens, depth + 1)?;
            Form::List(vec![Form::symbol(name), quoted])
        }
    };
    Ok(form)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        assert!(matches!(parse(""), Err(Error::IncompleteExpression(_))));
        assert!(matches!(
            parse("            "),
            Err(Error::IncompleteExpression(_))
        ));
    }

    #[test]
    fn parse_script_top_level_forms() {
        assert_eq!(
            parse_script("(def x 1)\n(+ x 2)"),
            Ok(vec![
                Form::List(vec![Form::symbol("def"), Form::symbol("x"), Form::Int(1)]),
                Form::List(vec![Form::symbol("+"), Form::symbol("x"), Form::Int(2)]),
            ])
        );
        assert_eq!(parse_script("# only a comment\n"), Ok(vec![]));
    }

    #[test]
    fn parse_nil() {
        assert_eq!(parse("nil"), Ok(Form::Nil));
    }

    #[test]
    fn parse_bool() {
        assert_eq!(parse("true"), Ok(Form::Bool(true)));
        assert_eq!(parse("false"), Ok(Form::Bool(false)));
    }

    #[test]
    fn parse_int() {
        assert_eq!(parse("1"), Ok(Form::Int(1)));
        assert_eq!(parse("     1     "), Ok(Form::Int(1)));
        assert_eq!(parse("10"), Ok(Form::Int(10)),);
        assert_eq!(parse("0"), Ok(Form::Int(0)),);
        assert_eq!(parse("-10"), Ok(Form::Int(-10)),);
    }

    #[test]
    fn parse_symbol() {
        assert_eq!(parse("hello"), Ok(Form::symbol("hello")));
        assert_eq!(parse("    hello    "), Ok(Form::symbol("hello")));
        assert_eq!(parse("an_keyword"), Ok(Form::symbol("an_keyword")),);
    }

    #[test]
    fn parse_string() {
        assert_eq!(parse("\"\""), Ok(Form::string("")));

        assert_eq!(parse("\"hello\""), Ok(Form::string("hello")));
        assert_eq!(parse("      \"hello\"      "), Ok(Form::string("hello")));

        assert_eq!(
            parse("\"  hello  world\""),
            Ok(Form::string("  hello  world"))
        );
        assert_eq!(
            parse("      \"hello  world  \"      "),
            Ok(Form::string("hello  world  "))
        );
        assert_eq!(
            parse("\"hello :not_a_keyword\""),
            Ok(Form::string("hello :not_a_keyword"))
        );

        assert_eq!(
            parse("\"\"\"\n  line one\n  \"line two\" \\\\end\n  \"\"\""),
            Ok(Form::string("line one\n\"line two\" \\\\end\n"))
        );
    }

    #[test]
    fn parse_list() {
        assert_eq!(
            parse("(add 1 2 \"three\")"),
            Ok(Form::List(vec![
                Form::symbol("add"),
                Form::Int(1),
                Form::Int(2),
                Form::string("three"),
            ]))
        );

        assert_eq!(
            parse("      (add       1      2 \"three\" )"),
            Ok(Form::List(vec![
                Form::symbol("add"),
                Form::Int(1),
                Form::Int(2),
                Form::string("three"),
            ]))
        );

        assert_eq!(
            parse("(() ()     (( )) )"),
            Ok(Form::List(vec![
                Form::List(vec![]),
                Form::List(vec![]),
                Form::List(vec![Form::List(vec![]),]),
            ]))
        )
    }

    #[test]
    fn parse_keywords() {
        assert_eq!(parse(":a_keyword"), Ok(Form::keyword("a_keyword")));

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
    fn parse_nested() {
        assert_eq!(
            parse("(defun hello (x y z) (print \"hello\"))"),
            Ok(Form::List(vec![
                Form::symbol("defun"),
                Form::symbol("hello"),
                Form::List(vec![
                    Form::symbol("x"),
                    Form::symbol("y"),
                    Form::symbol("z"),
                ]),
                Form::List(vec![Form::symbol("print"), Form::string("hello"),]),
            ]),)
        );

        assert_eq!(
            parse("(hello (world (:a_keyword)) \"string\" 10 -99)"),
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

    #[test]
    fn parse_quoted() {
        assert_eq!(
            parse("'()"),
            Ok(Form::List(vec![Form::symbol("quote"), Form::List(vec![]),]))
        );
        assert_eq!(
            parse("'(1 2 3)"),
            Ok(Form::List(vec![
                Form::symbol("quote"),
                Form::List(vec![Form::Int(1), Form::Int(2), Form::Int(3),]),
            ]))
        );
        assert_eq!(
            parse("(hello '(1 2 3))"),
            Ok(Form::List(vec![
                Form::symbol("hello"),
                Form::List(vec![
                    Form::symbol("quote"),
                    Form::List(vec![Form::Int(1), Form::Int(2), Form::Int(3),]),
                ])
            ]))
        );

        assert_eq!(
            parse("'(hello '(1 2 3))"),
            Ok(Form::List(vec![
                Form::symbol("quote"),
                Form::List(vec![
                    Form::symbol("hello"),
                    Form::List(vec![
                        Form::symbol("quote"),
                        Form::List(vec![Form::Int(1), Form::Int(2), Form::Int(3),]),
                    ])
                ])
            ]))
        )
    }

    #[test]
    fn parse_partial_form() {
        assert!(
            matches!(parse("1 2 3"), Err(Error::IncompleteExpression(_))),
            "parse should fail if entire expression cannot be consumed as single form"
        );
    }

    #[test]
    fn parse_unterminated_list() {
        assert!(matches!(
            parse("(1 2 3"),
            Err(Error::IncompleteExpression(_))
        ));
    }
}
