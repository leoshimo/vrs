//! Service Bindings
//! See also [super::registry]

use lyric::builtin::cond::is_true;
use lyric::{compile, kwargs, parse, Error, KeywordId, Result, SymbolId};

use crate::rt::program::{Fiber, Lambda, NativeAsyncFn, NativeFn, NativeFnOp, Val};
use crate::rt::registry::Registration;

/// Binding for register
pub(crate) fn register_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        metadata: vec![],
        doc: "(register SVC_NAME [:interface INTERFACE]) - \
              Register caller as SVC_NAME in service registry, optionally providing \
              INTERFACE keyword argument for publishing available interface."
            .to_string(),
        func: |f, args| Box::new(register_impl(f, args)),
    }
}

/// Implementation for (register NAME [:interface INTERFACE_LIST] [:overwrite])
async fn register_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let keyword = match args.first() {
        Some(Val::Keyword(k)) => k.clone(),
        _ => {
            return Err(Error::UnexpectedArguments(
                "register expects a keyword argument as first argument".to_string(),
            ))
        }
    };

    let mut reg = Registration::new(keyword);

    if let Some(interface) = kwargs::get(&args[1..], &KeywordId::from("interface")) {
        let symbols = match interface {
            Val::List(ref symbols) => Ok(symbols),
            _ => Err(Error::UnexpectedArguments(
                ":interface keyword argument must be a list".to_string(),
            )),
        }?;
        let symbols = symbols
            .iter()
            .map(|e| match e {
                Val::Symbol(s) => Ok(s),
                _ => Err(Error::UnexpectedArguments(
                    "Forms in :interface list should be symbols".to_string(),
                )),
            })
            .collect::<Result<Vec<_>>>()?;

        let env = fiber.cur_env().lock().unwrap();
        let mut interface = vec![];
        for sym in &symbols {
            let val = env.get(sym).ok_or(Error::InvalidExpression(format!(
                "No symbol bound to {}",
                sym
            )))?;
            let lambda = match val {
                Val::Lambda(l) => Ok(l),
                _ => Err(Error::UnexpectedArguments(format!(
                    "{} is not a lambda - found {}",
                    sym, val
                ))),
            }?;

            let pattern = lambda_interface(sym, &lambda);
            interface.push(pattern.clone());
            reg.metadata((*sym).clone().to_keyword(), lambda.metadata.clone());

            if let Some(doc) = lambda.doc {
                reg.docs((*sym).clone().to_keyword(), doc);
            }
        }

        reg.interface(interface.clone());
        let mut completions = env.entity_completions();
        for providers in completions.values_mut() {
            providers.retain(|provider| symbols.contains(&provider));
        }
        completions.retain(|_, providers| !providers.is_empty());
        reg.entity_completions(completions);
    }

    let overwrite_flag =
        kwargs::flag(&args[1..], &KeywordId::from("overwrite")).unwrap_or(Val::Bool(false));
    if is_true(&overwrite_flag)? {
        reg.overwrite(true);
    }

    let hdl = fiber
        .locals()
        .self_handle
        .as_ref()
        .expect("process should have self handle");

    let registry = fiber
        .locals()
        .registry
        .as_ref()
        .expect("process should have registry handle");

    registry
        .register(reg, hdl.clone())
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;

    Ok(Val::keyword("ok"))
}

/// Binding for ls_srv
pub(crate) fn ls_srv_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        metadata: vec![],
        doc: "(ls_srv) - Returns the selected service and exported interface for every registered name"
            .to_string(),
        func: |f, args| Box::new(ls_srv_impl(f, args)),
    }
}

/// Implementation of (ls_srv)
async fn ls_srv_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    if !args.is_empty() {
        return Err(Error::UnexpectedArguments(
            "ls_srv expects no arguments".to_string(),
        ));
    }

    let registry = fiber
        .locals()
        .registry
        .as_ref()
        .expect("process should have registry handle");

    let entries = registry
        .all()
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;

    let mut entry_values: Vec<_> = vec![];

    for e in entries {
        let val = Val::from(e);
        let name = kwargs::get(val.as_list()?, &KeywordId::from("name")).ok_or(Error::Runtime(
            "service entry did not contain service name".to_string(),
        ))?;
        entry_values.push(name);
        entry_values.push(val.clone());
    }

    Ok(Val::List(entry_values))
}

/// Binding for find_srv
pub(crate) fn find_srv_fn() -> Lambda {
    Lambda {
        metadata: vec![],
        doc: Some(
            "(find_srv SVC_NAME) - Returns the process id of SVC_NAME in the service registry. \
              Raises an error if SVC_NAME is not registered."
                .to_string(),
        ),
        params: vec![SymbolId::from("srv_name")],
        code: compile(&parse("(info_srv srv_name :pid)").unwrap().into()).unwrap(),
        parent: None,
    }
}

/// Binding for info_srv
pub(crate) fn info_srv_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        metadata: vec![],
        doc: "(info_srv SVC_NAME ATTR) - Returns the attribute ATTR for process registered as SVC_NAME in service registry.".to_string(),
        func: |f, args| Box::new(info_srv_impl(f, args)),
    }
}

/// Implementation for (info_srv NAME ATTR)
async fn info_srv_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let (keyword, query) = match &args[..] {
        [Val::Keyword(k), Val::Keyword(q)] => (k, q),
        _ => {
            return Err(Error::UnexpectedArguments(
                "info_srv expects single keyword argument".to_string(),
            ))
        }
    };

    let entry = fiber
        .locals()
        .registry
        .as_ref()
        .expect("no registry for process")
        .lookup(keyword.clone())
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?
        .ok_or(Error::Runtime(format!("No service found for {keyword}")))?;

    match query.as_str() {
        "pid" => Ok(entry.process_val()),
        "interface" => Ok(Val::List(entry.interface().to_vec())),
        "interface_doc" => {
            let mut interface_doc = vec![];
            for i in entry.interface() {
                let kwd = i
                    .as_list()?
                    .first()
                    .ok_or(Error::Runtime(
                        "empty signature in interface list found".to_string(),
                    ))?
                    .as_keyword()?;

                let doc = match entry.doc(kwd) {
                    Some(doc) => doc.to_string(),
                    None => format!("<no documentation for {}>", kwd.clone().to_symbol()),
                };
                interface_doc.push(Val::List(vec![
                    Val::keyword("interface"),
                    i.clone(),
                    Val::keyword("doc"),
                    Val::String(doc),
                    Val::keyword("metadata"),
                    Val::from(lyric::Form::List(entry.metadata(kwd))),
                ]));
            }

            Ok(Val::List(interface_doc))
        }
        "entity_completions" => {
            let mut types = entry.entity_completions().iter().collect::<Vec<_>>();
            types.sort_by(|(a, _), (b, _)| a.as_str().cmp(b.as_str()));
            Ok(Val::List(
                types
                    .into_iter()
                    .flat_map(|(ty, providers)| {
                        [
                            Val::Keyword(ty.clone()),
                            Val::List(providers.iter().cloned().map(Val::Symbol).collect()),
                        ]
                    })
                    .collect(),
            ))
        }
        q => Err(Error::UnexpectedArguments(format!(
            "info_srv got unexpected query: {}",
            q
        ))),
    }
}

pub(crate) fn import_entity_completions_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "Internal bind_srv helper: replace one service's default completion associations"
            .into(),
        func: |fiber, args| {
            let [Val::Keyword(service), Val::List(record)] = args else {
                return Err(Error::UnexpectedArguments(
                    "invalid imported completion associations".into(),
                ));
            };
            if record.len() % 2 != 0 {
                return Err(Error::UnexpectedArguments(
                    "invalid completion record".into(),
                ));
            }
            let mut completions = lyric::env::EntityCompletions::new();
            for pair in record.chunks_exact(2) {
                let ty = pair[0].as_keyword()?.clone();
                let providers = pair[1]
                    .as_list()?
                    .iter()
                    .map(|value| value.as_symbol().cloned())
                    .collect::<Result<Vec<_>>>()?;
                completions.insert(ty, providers);
            }
            fiber
                .global_env()
                .lock()
                .unwrap()
                .import_entity_completions(service.clone(), completions);
            Ok(NativeFnOp::Return(Val::keyword("ok")))
        },
    }
}

/// Keep protocol type checks in the runtime; dispatch remains visible Lyric.
fn service_request_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "Internal service helper: whether a message is a call request envelope".into(),
        func: |_, args| match args {
            [Val::List(message)] => Ok(NativeFnOp::Return(Val::Bool(matches!(
                message.as_slice(),
                [Val::Ref(_), Val::Extern(crate::Extern::ProcessId(_)), _]
            )))),
            [_] => Ok(NativeFnOp::Return(Val::Bool(false))),
            _ => Err(Error::UnexpectedArguments("expected one message".into())),
        },
    }
}

fn service_event_error_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "Internal service helper: report an event handler error without sending a reply"
            .into(),
        func: |fiber, args| {
            let [service, topic, error] = args else {
                return Err(Error::UnexpectedArguments(
                    "expected service, topic, and error".into(),
                ));
            };
            tracing::error!(pid = %fiber.locals().pid, %service, %topic, %error, "service event handler failed");
            Ok(NativeFnOp::Return(Val::Nil))
        },
    }
}

/// Cache the standard source library, then install its global-lookup lambdas
/// into each process. No registry operations execute while loading definitions.
pub(crate) fn install_service_library(env: &mut crate::Env) {
    use std::sync::OnceLock;
    env.bind_native(SymbolId::from("vrs/service_request?"), service_request_fn());
    env.bind_native(
        SymbolId::from("vrs/report_service_event_error"),
        service_event_error_fn(),
    );
    type Macros = lyric::macros::MacroEnv<crate::Extern, crate::Locals>;
    static LIBRARY: OnceLock<(Macros, Vec<(SymbolId, Val)>)> = OnceLock::new();
    let (macros, definitions) = LIBRARY.get_or_init(|| {
        let source = concat!(
            include_str!("../stdlib/remote.ll"),
            "\n",
            include_str!("../stdlib/service-macros.ll"),
            "\n",
            include_str!("../stdlib/services.ll"),
            "\n",
            include_str!("../stdlib/gui.ll"),
            "\n",
            include_str!("../stdlib/commands.ll"),
            "\n",
            include_str!("../stdlib/editor.ll")
        );
        let forms = lyric::parse_script(source).expect("standard service library must parse");
        let names: Vec<_> = forms
            .iter()
            .filter(|form| matches!(form, lyric::Form::List(items) if items.first() == Some(&lyric::Form::symbol("defn!"))))
            .map(|form| match form {
                lyric::Form::List(items) => match &items[1] {
                    lyric::Form::Symbol(name) => name.clone(),
                    _ => panic!("library definition needs a name"),
                },
                _ => panic!("library definition must be a list"),
            })
            .collect();
        let body = Val::List(
            std::iter::once(Val::symbol("begin"))
                .chain(forms.into_iter().map(Val::from))
                .collect(),
        );
        let mut fiber = Fiber::from_val(
            &body,
            crate::Env::standard(),
            crate::Locals::new(crate::ProcessId::new("library", 0)),
        )
        .unwrap();
        assert!(matches!(
            fiber
                .start()
                .expect("service library definitions must evaluate"),
            lyric::Signal::Done(_)
        ));
        let root = fiber.global_env().lock().unwrap();
        let definitions = names
            .into_iter()
            .map(|name| {
                let mut value = root.get(&name).expect("library definition must exist");
                if let Val::Lambda(lambda) = &mut value {
                    lambda.parent = None;
                }
                (name, value)
            })
            .collect();
        let mut macros = root.macro_env();
        macros.use_global_scope();
        (macros, definitions)
    });
    env.set_macro_env(macros.clone());
    for (name, value) in definitions {
        env.define(name.clone(), value.clone());
    }
}

/// Generates interface for calling exported lambda
fn lambda_interface(symbol: &SymbolId, lambda: &Lambda) -> Val {
    Val::List(
        std::iter::once(Val::Keyword(symbol.clone().to_keyword()))
            .chain(lambda.params.iter().map(|v| Val::Symbol(v.clone())))
            .collect::<Vec<_>>(),
    )
}
