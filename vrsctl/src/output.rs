//! Shared output policy for commands, files/stdin, the REPL, and subscriptions.
use lyric::Form;
use std::io::{self, IsTerminal, Write};

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq)]
pub(crate) enum Format {
    #[clap(help = "Pretty on terminal stdout, compact when redirected")]
    Default,
    #[clap(help = "Compact Lyric representation (also on terminals)")]
    Compact,
    #[clap(help = "Multiline Lyric representation at the target width")]
    Pretty,
    #[clap(help = "Echo source and comment every line of the pretty result")]
    Editor,
}

pub(crate) struct Output {
    pub(crate) format: Format,
    pub(crate) width: Option<usize>,
    pub(crate) raw: bool,
    pub(crate) terminal: bool,
}

impl Output {
    pub(crate) fn new(format: Format, width: Option<usize>, raw: bool) -> Self {
        Self {
            format,
            width,
            raw,
            terminal: io::stdout().is_terminal(),
        }
    }

    fn width(&self) -> usize {
        // Query for each result so following subscriptions and the REPL resize.
        self.width
            .or_else(|| {
                terminal_size::terminal_size_of(io::stdout())
                    .map(|(terminal_size::Width(width), _)| usize::from(width))
                    .filter(|width| *width > 0)
            })
            .unwrap_or(80)
    }

    pub(crate) fn render(&self, form: &Form) -> String {
        if self.raw {
            if let Form::String(text) = form {
                return text.clone();
            }
        }
        match self.format {
            Format::Compact => form.to_string(),
            Format::Default if !self.terminal => form.to_string(),
            Format::Editor => form.to_pretty_string(self.width().saturating_sub(5)),
            _ => form.to_pretty_string(self.width()),
        }
    }

    pub(crate) fn write(
        &self,
        writer: &mut impl Write,
        form: &Form,
        source: &str,
    ) -> io::Result<()> {
        let text = self.render(form);
        if self.format == Format::Editor {
            write!(writer, "{source}")?;
            if !source.is_empty() && !source.ends_with('\n') {
                writeln!(writer)?;
            }
            // Raw strings can contain newlines too. Never emit uncommented
            // continuation lines into an editor transcript.
            for (index, line) in text.split('\n').enumerate() {
                writeln!(
                    writer,
                    "{}{}",
                    if index == 0 { "# => " } else { "#    " },
                    line
                )?;
            }
        } else {
            writeln!(writer, "{text}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_default_and_explicit_formats() {
        let form = Form::from_expr("((1 2) (3 4))").unwrap();
        for (format, terminal, expected) in [
            (Format::Default, true, "((1 2)\n (3 4))"),
            (Format::Default, false, "((1 2) (3 4))"),
            (Format::Compact, true, "((1 2) (3 4))"),
            (Format::Pretty, false, "((1 2)\n (3 4))"),
        ] {
            let output = Output {
                format,
                terminal,
                width: Some(10),
                raw: false,
            };
            assert_eq!(output.render(&form), expected);
        }
    }

    #[test]
    fn strings_stay_escaped_unless_raw_is_requested() {
        let form = Form::string("one\n\"two\"\\three");
        let mut output = Output::new(Format::Pretty, Some(10), false);
        assert_eq!(output.render(&form), form.to_string());
        output.raw = true;
        assert_eq!(output.render(&form), "one\n\"two\"\\three");
        let nested = Form::List(vec![form.clone()]);
        assert_eq!(lyric::parse(&output.render(&nested)).unwrap(), nested);
        let raw = Form::RawString("raw\n  text".into());
        assert_eq!(output.render(&raw), "raw\n  text");
    }

    #[test]
    fn editor_comments_every_result_line_and_separates_unterminated_source() {
        let output = Output::new(Format::Editor, Some(16), false);
        let form = Form::from_expr("((1 2) (3 4))").unwrap();
        let mut buf = Vec::new();
        output.write(&mut buf, &form, "(list 1 2)").unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "(list 1 2)\n# => ((1 2)\n#     (3 4))\n"
        );
        let mut buf = Vec::new();
        output
            .write(&mut buf, &Form::RawString("a\n(danger)\n".into()), "")
            .unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "# => a\n#    (danger)\n#    \n"
        );
    }
}
