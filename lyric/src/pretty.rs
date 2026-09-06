//! A value printer, separate from the compact Display/wire representation.
use crate::{Extern, Form, Locals, Val};
use std::fmt::Display;
use unicode_width::UnicodeWidthStr;

pub(crate) trait Value: Display + Sized {
    fn list(&self) -> Option<&[Self]>;
    fn symbol(&self) -> Option<&str>;
    fn is_keyword(&self) -> bool;
}

/// Shared by compact and pretty printers so prefix tokenization stays identical.
pub(crate) fn abbreviation<V: Value>(items: &[V]) -> Option<(&'static str, &V)> {
    let [head, value] = items else { return None };
    let prefix = match head.symbol()? {
        "quote" => "'",
        "quasiquote" => "`",
        "unquote" if value.symbol().is_some_and(|name| name.starts_with('@')) => ", ",
        "unquote" => ",",
        "unquote-splicing" => ",@",
        _ => return None,
    };
    Some((prefix, value))
}

impl Value for Form {
    fn list(&self) -> Option<&[Self]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }
    fn symbol(&self) -> Option<&str> {
        match self {
            Self::Symbol(id) => Some(id.as_str()),
            _ => None,
        }
    }
    fn is_keyword(&self) -> bool {
        matches!(self, Self::Keyword(_))
    }
}

impl<T: Extern, L: Locals> Value for Val<T, L> {
    fn list(&self) -> Option<&[Self]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }
    fn symbol(&self) -> Option<&str> {
        match self {
            Self::Symbol(id) => Some(id.as_str()),
            _ => None,
        }
    }
    fn is_keyword(&self) -> bool {
        matches!(self, Self::Keyword(_))
    }
}

struct Doc {
    // Cache flat widths once, avoiding repeated printing of large subtrees.
    flat_width: usize,
    kind: Kind,
}

enum Kind {
    Atom(String),
    Prefix(&'static str, Box<Doc>),
    List {
        items: Vec<Doc>,
        indent: usize,
        pairs: bool,
    },
}

#[derive(Clone, Copy, Default)]
struct Context {
    data: bool,
    literal: bool,
    depth: usize,
}

impl Context {
    fn prefixed(self, prefix: &str) -> Self {
        if self.literal {
            return self;
        }
        match prefix {
            "'" if self.depth == 0 => Self {
                literal: true,
                data: true,
                ..self
            },
            "`" => Self {
                depth: self.depth + 1,
                data: true,
                ..self
            },
            "," | ", " | ",@" if self.depth > 0 => Self {
                depth: self.depth - 1,
                data: self.depth > 1,
                ..self
            },
            _ => self,
        }
    }
}

impl Doc {
    fn new(value: &impl Value, context: Context) -> Self {
        if let Some(items) = value.list() {
            if let Some((prefix, value)) = abbreviation(items) {
                let doc = Self::new(value, context.prefixed(prefix));
                return Self {
                    flat_width: doc.flat_width.saturating_add(prefix.len()),
                    kind: Kind::Prefix(prefix, Box::new(doc)),
                };
            }
            let pairs = !items.is_empty()
                && items.len() % 2 == 0
                && items.iter().step_by(2).all(Value::is_keyword);
            let quoted = context.literal || context.data || context.depth > 0;
            let indent = if !quoted && items.first().and_then(Value::symbol).is_some() {
                2
            } else {
                1
            };
            let data = quoted || items.first().and_then(Value::symbol).is_none();
            let items: Vec<_> = items
                .iter()
                .map(|item| Self::new(item, Context { data, ..context }))
                .collect();
            let flat_width = items
                .iter()
                .fold(2 + items.len().saturating_sub(1), |width, item| {
                    width.saturating_add(item.flat_width)
                });
            Self {
                flat_width,
                kind: Kind::List {
                    items,
                    indent,
                    pairs,
                },
            }
        } else {
            let text = value.to_string();
            let flat_width = if text.contains(['\n', '\r']) {
                usize::MAX
            } else {
                text.width()
            };
            Self {
                flat_width,
                kind: Kind::Atom(text),
            }
        }
    }

    fn flat(&self, out: &mut String) {
        match &self.kind {
            Kind::Atom(text) => out.push_str(text),
            Kind::Prefix(prefix, doc) => {
                out.push_str(prefix);
                doc.flat(out);
            }
            Kind::List { items, .. } => {
                out.push('(');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(' ');
                    }
                    item.flat(out);
                }
                out.push(')');
            }
        }
    }

    fn can_break(&self) -> bool {
        match &self.kind {
            Kind::Atom(_) => false,
            Kind::Prefix(_, doc) => doc.can_break(),
            Kind::List { items, .. } => !items.is_empty(),
        }
    }

    // `tail` reserves space for closing delimiters on the last line.
    fn render(&self, out: &mut String, width: usize, column: usize, tail: usize) -> usize {
        if self.flat_width <= width.saturating_sub(column.saturating_add(tail)) {
            self.flat(out);
            return column + self.flat_width;
        }
        match &self.kind {
            Kind::Atom(text) => {
                out.push_str(text);
                match text.rsplit_once('\n') {
                    Some((_, last)) => last.width(),
                    None => column.saturating_add(text.width()),
                }
            }
            Kind::Prefix(prefix, doc) => {
                out.push_str(prefix);
                doc.render(out, width, column + prefix.len(), tail)
            }
            Kind::List {
                items,
                indent,
                pairs,
            } => {
                out.push('(');
                let mut current = column + 1;
                for (index, item) in items.iter().enumerate() {
                    let tail = if index + 1 == items.len() {
                        tail + 1
                    } else {
                        0
                    };
                    if index > 0 {
                        let fits = current
                            .saturating_add(1)
                            .saturating_add(item.flat_width)
                            .saturating_add(tail)
                            <= width;
                        if *pairs && index % 2 == 1 && (fits || !item.can_break()) {
                            out.push(' ');
                            current += 1;
                        } else {
                            out.push('\n');
                            current = column + indent;
                            out.extend(std::iter::repeat_n(' ', current));
                        }
                    }
                    current = item.render(out, width, current, tail);
                }
                out.push(')');
                current + 1
            }
        }
    }
}

pub(crate) fn format(value: &impl Value, width: usize) -> String {
    let mut out = String::new();
    Doc::new(value, Context::default()).render(&mut out, width.max(1), 0, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestVal = Val<void::Void, ()>;

    #[test]
    fn width_boundaries_and_closing_delimiters() {
        let form = Form::from_expr("((1 2) (3 4))").unwrap();
        assert_eq!(form.to_pretty_string(13), form.to_string());
        assert_eq!(form.to_pretty_string(12), "((1 2)\n (3 4))");
        assert_eq!(form.to_pretty_string(11), "((1 2)\n (3 4))");
        assert_eq!(form.to_pretty_string(6), "((1 2)\n (3\n  4))");
    }

    #[test]
    fn service_records_keep_pairs_and_nested_rows_together() {
        let form = Form::from_expr("((:name :echo :node \"alpha\" :interface ((ping x) (pong y))) (:name :clock :interface ((now))))").unwrap();
        assert_eq!(
            form.to_pretty_string(40),
            concat!(
                "((:name :echo\n",
                "  :node \"alpha\"\n",
                "  :interface ((ping x) (pong y)))\n",
                " (:name :clock :interface ((now))))"
            )
        );
    }

    #[test]
    fn large_compound_values_start_below_their_keyword() {
        let form = Form::from_expr(
            "(:echo (:name :echo :interface ((ping x) (pong y))) :clock (:name :clock))",
        )
        .unwrap();
        assert_eq!(
            form.to_pretty_string(35),
            concat!(
                "(:echo\n",
                " (:name :echo\n",
                "  :interface ((ping x) (pong y)))\n",
                " :clock (:name :clock))"
            )
        );
        assert_eq!(Form::from_expr(&form.to_pretty_string(35)).unwrap(), form);
    }

    #[test]
    fn serializable_values_round_trip_at_all_widths() {
        for source in [
            "nil",
            "true",
            "false",
            "-123",
            "()",
            ":key",
            "symbol",
            r#""line\nquote\"slash\\tab\treturn\r""#,
            "'(alpha (beta gamma) '(delta epsilon))",
            "(:a 1 :b (:c (1 2 3)) :d ())",
            "(:odd 1 :last)",
            "(quote)",
            "(quote a b)",
            "(\"日本語\" \"é\" \"🙂\")",
            "\"\"\"\n    embedded \"quote\" and \\slash\n    \"\"\"",
        ] {
            let form = Form::from_expr(source).unwrap();
            let value: TestVal = form.clone().into();
            for width in [0, 1, 5, 12, 40, 80, usize::MAX] {
                let formatted = form.to_pretty_string(width);
                assert_eq!(
                    Form::from_expr(&formatted).unwrap(),
                    form,
                    "{source}: {width}"
                );
                assert_eq!(value.to_pretty_string(width), formatted);
            }
        }
    }

    #[test]
    fn unicode_uses_display_columns_and_atoms_are_not_split() {
        let form = Form::from_expr("(\"日本\" 1)").unwrap();
        assert_eq!(form.to_pretty_string(10), "(\"日本\" 1)");
        assert_eq!(form.to_pretty_string(9), "(\"日本\"\n 1)");
        assert_eq!(form.to_pretty_string(1), "(\"日本\"\n 1)");
    }

    #[test]
    fn raw_and_opaque_values_preserve_display() {
        let raw = Form::RawString("line one\n  line two\n".into());
        assert_eq!(raw.to_pretty_string(1), raw.to_string());
        let value: TestVal = Val::List(vec![Val::keyword("ref"), Val::Ref(crate::Ref("7".into()))]);
        let form: Form = value.clone().try_into().unwrap();
        assert_eq!(value.to_pretty_string(1), "(:ref <ref 7>)");
        assert_eq!(form.to_pretty_string(1), value.to_pretty_string(1));
    }

    #[test]
    fn large_collection_is_not_truncated() {
        let form = Form::List((0..10_000).map(Form::Int).collect());
        let text = form.to_pretty_string(80);
        assert_eq!(Form::from_expr(&text).unwrap(), form);
        assert!(text.lines().all(|line| line.width() <= 80));
    }
}
