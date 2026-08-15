//! Compile datum templates without evaluating holes outside the ordinary VM.
use crate::{compile as compile_expr, Bytecode, Error, Extern, Inst, Locals, Result, Val};

pub(crate) fn compile<T: Extern, L: Locals>(args: &[Val<T, L>]) -> Result<Bytecode<T, L>> {
    let [template] = args else {
        return Err(Error::InvalidExpression(
            "quasiquote expects exactly one argument".into(),
        ));
    };
    let mut code = Vec::new();
    value(template, 1, &mut code)?;
    Ok(code)
}

fn marker<T: Extern, L: Locals>(form: &Val<T, L>) -> Result<Option<(&str, &Val<T, L>)>> {
    if let Val::List(items) = form {
        if let Some(Val::Symbol(head)) = items.first() {
            let name = head.as_str();
            if matches!(name, "quasiquote" | "unquote" | "unquote-splicing") {
                return match &items[1..] {
                    [operand] => Ok(Some((name, operand))),
                    _ => Err(Error::InvalidExpression(format!(
                        "{name} expects exactly one argument"
                    ))),
                };
            }
        }
    }
    Ok(None)
}

fn value<T: Extern, L: Locals>(
    form: &Val<T, L>,
    depth: usize,
    code: &mut Bytecode<T, L>,
) -> Result<()> {
    if let Some((name, operand)) = marker(form)? {
        let inner_depth = match (name, depth) {
            ("unquote", 1) => {
                code.extend(compile_expr(operand)?);
                return Ok(());
            }
            ("unquote-splicing", 1) => {
                return Err(Error::InvalidExpression(
                    "unquote-splicing requires a list-element position".into(),
                ));
            }
            ("quasiquote", _) => depth + 1,
            _ => depth - 1,
        };
        // Preserve the marker, processing its one operand as a single value.
        code.push(Inst::PushConst(Val::List(vec![Val::symbol(name)])));
        value(operand, inner_depth, code)?;
        code.push(Inst::ListPush);
    } else if let Val::List(items) = form {
        code.push(Inst::PushConst(Val::List(vec![])));
        for item in items {
            if let Some(("unquote-splicing", operand)) = marker(item)? {
                if depth == 1 {
                    code.extend(compile_expr(operand)?);
                    code.push(Inst::ListExtend);
                    continue;
                }
            }
            // `quote` is ordinary structure here: ',x captures a quoted value.
            value(item, depth, code)?;
            code.push(Inst::ListPush);
        }
    } else {
        code.push(Inst::PushConst(form.clone()));
    }
    Ok(())
}
