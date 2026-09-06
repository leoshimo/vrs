use lyric::{Env, Error, Fiber, Form, NativeAsyncFn, Signal, SymbolId, Val};
use void::Void;

type Value = Val<Void, ()>;

fn eval(source: &str) -> lyric::Result<Value> {
    let mut fiber = Fiber::from_expr(source, Env::standard(), ())?;
    match fiber.start()? {
        Signal::Done(value) => Ok(value),
        other => panic!("unexpected suspension: {other:?}"),
    }
}

fn assert_value(source: &str, expected: &str) {
    assert_eq!(
        eval(source).unwrap(),
        Value::from_expr(expected).unwrap(),
        "{source}"
    );
}

#[test]
fn reader_desugars_prefixes_without_context_or_execution() {
    for (short, long) in [
        ("'x", "(quote x)"),
        ("`x", "(quasiquote x)"),
        (",x", "(unquote x)"),
        (",@x", "(unquote-splicing x)"),
        ("',x", "(quote (unquote x))"),
        (
            "`(a ,b ,@c)",
            "(quasiquote (a (unquote b) (unquote-splicing c)))",
        ),
        (", @x", "(unquote @x)"),
        (
            "` # comment\n (a , # another\n x)",
            "(quasiquote (a (unquote x)))",
        ),
        (
            ",@(error \"must not run\")",
            "(unquote-splicing (error \"must not run\"))",
        ),
    ] {
        assert_eq!(lyric::parse(short), lyric::parse(long), "{short}");
    }
    assert_eq!(
        lyric::parse_script("x,y`z").unwrap(),
        vec![
            Form::symbol("x"),
            lyric::parse(",y").unwrap(),
            lyric::parse("`z").unwrap()
        ]
    );
    assert_value("(read \",x\")", "(unquote x)");
}

#[test]
fn prefixes_require_one_following_form() {
    for prefix in ["'", "`", ",", ",@"] {
        assert!(
            matches!(lyric::parse(prefix), Err(Error::IncompleteExpression(message)) if message.contains(prefix))
        );
        assert!(
            matches!(lyric::parse(&format!("({prefix})")), Err(Error::InvalidExpression(message)) if message.contains(prefix))
        );
    }
}

#[test]
fn strings_and_comments_do_not_interpolate() {
    assert_value(
        r#"`("literal ,x ` ,@xs" """raw ,x ` ,@xs""")"#,
        r#"("literal ,x ` ,@xs" "raw ,x ` ,@xs")"#,
    );
    assert_value("`(a # ,@missing\n b)", "(a b)");
}

#[test]
fn atoms_nested_lists_and_splices_are_values() {
    assert_value("`missing", "missing");
    assert_value("`nil", "nil");
    assert_value("`true", "true");
    assert_value("`42", "42");
    assert_value("`()", "()");
    assert_value("`(+ 1 2)", "(+ 1 2)");
    assert_value(
        "(let ((x 7) (xs '(8 9))) `(a ,x ,xs ,@xs))",
        "(a 7 (8 9) 8 9)",
    );
    assert_value("(let ((x 7)) `,x)", "7");
    assert_value("`(a ,nil (b ,(+ 2 3)))", "(a nil (b 5))");
    assert_value("`(,@'(f) ,@'() 1 ,@'((2 3) 4))", "(f 1 (2 3) 4)");
    assert_value("`(,@'())", "()");
}

#[test]
fn nesting_preserves_inactive_markers_and_activates_matching_depth() {
    assert_value(
        "(let ((x 7)) `(outer `(inner ,x ,,x)))",
        "(outer (quasiquote (inner (unquote x) (unquote 7))))",
    );
    assert_value(
        "`(outer `(inner ,@missing))",
        "(outer (quasiquote (inner (unquote-splicing missing))))",
    );
    assert_value(
        "(let ((xs '(a b))) `(outer `(inner ,@,xs)))",
        "(outer (quasiquote (inner (unquote-splicing (a b)))))",
    );
    assert_value("(let ((x 7)) `(a ,`(b ,x)))", "(a (b 7))");
    assert_value("(let ((x 7)) (eval ``(inner ,x)))", "(inner 7)");
}

#[test]
fn quote_inside_a_template_preserves_future_literal_arguments() {
    assert_value("(let ((name 'focus_window) (window '(:os/window :id 42))) `(continue_call ',name '(,window)))",
                 "(continue_call (quote focus_window) (quote ((:os/window :id 42))))");
    assert_value(
        "'(quasiquote (a ,missing))",
        "(quasiquote (a (unquote missing)))",
    );
    assert_value("'(unquote)", "(unquote)");
    assert_value(
        "`(tag ,'(unquote missing) ,@'((unquote missing)))",
        "(tag (unquote missing) (unquote missing))",
    );
    assert_value("`(quote ,7)", "(quote 7)");
    assert_value("`(quote ,@'(1 2))", "(quote 1 2)");
}

#[test]
fn holes_run_once_in_lexical_order_and_template_code_runs_later() {
    assert_value(
        "(begin
      (def calls '())
      (defn mark (x) (set calls (push calls x)) x)
      (def code (let ((x 2)) `(mark ,(mark 1) (nested ,(mark x)) ,@(list (mark 3)))))
      (list calls code))",
        "((1 2 3) (mark 1 (nested 2) 3))",
    );
    assert_value(
        "(begin
      (def calls '()) (defn mark (x) (set calls (push calls x)))
      (def code `(begin (mark ,1) (mark ,2)))
      (def before calls) (eval code) (list before calls))",
        "(() (1 2))",
    );
    assert_value(
        "(begin (def x 7) (def code `(list ,x x)) (set x 8) (eval code))",
        "(7 8)",
    );
}

#[test]
fn compiler_construction_does_not_use_rebound_helpers_or_mutate_inputs() {
    assert_value(
        "(let ((list 1) (+ 2) (xs '(1 2))) `(a ,@xs ,xs))",
        "(a 1 2 (1 2))",
    );
    assert_value(
        "(begin (def xs '(1 2)) (def ys `(0 ,@xs 3)) (list xs ys))",
        "((1 2) (0 1 2 3))",
    );
    assert_value("(let ((list 1)) (quasiquote (a (unquote 7))))", "(a 7)");
}

#[test]
fn marker_errors_are_compiler_errors_for_reader_and_constructed_forms() {
    for source in [
        ",x",
        ",@x",
        "` ,@xs",
        "(quasiquote)",
        "(quasiquote a b)",
        "`((unquote))",
        "`((unquote 1 2))",
        "`((unquote-splicing))",
        "`((quasiquote a b))",
        "``(,,@xs)",
    ] {
        assert!(
            matches!(eval(source), Err(Error::InvalidExpression(_))),
            "{source}"
        );
    }
    assert!(matches!(
        eval("(eval (list 'unquote 1))"),
        Err(Error::InvalidExpression(_))
    ));
    assert!(matches!(
        eval("(eval `(,@'()))"),
        Err(Error::InvalidExpression(_))
    ));
}

#[test]
fn invalid_splices_stop_before_later_holes_and_unwind_builders() {
    for value in ["nil", "1", "\"abc\"", "true", "'symbol"] {
        assert!(
            matches!(eval(&format!("`(a ,@{value} b)")), Err(Error::UnexpectedArguments(message)) if message.contains("expects a list"))
        );
    }
    assert_value(
        "(begin
      (def calls '())
      (def result (try `(a ,(set calls '(before)) (nested ,@nil ,(set calls '(after))))))
      (list (err? result) calls `(still ,(+ 1 2))))",
        "(true (before) (still 3))",
    );
    assert_value("(begin (def value `(a ,(try (error \"hole\")) ,(+ 1 2))) (list (err? (get value 1)) (get value 2)))", "(true 3)");
}

#[test]
fn list_builders_survive_yield_and_eval() {
    let mut fiber = Fiber::<Void, ()>::from_expr(
        "`(a ,(yield 1) ,@(yield 2) ,(eval '(+ 2 3)))",
        Env::standard(),
        (),
    )
    .unwrap();
    assert!(matches!(
        fiber.start().unwrap(),
        Signal::Yield(Value::Int(1))
    ));
    assert!(matches!(
        fiber.resume(Ok(Value::Int(7))).unwrap(),
        Signal::Yield(Value::Int(2))
    ));
    assert!(
        matches!(fiber.resume(Ok(Value::from_expr("(8 9)").unwrap())).unwrap(), Signal::Done(value) if value == Value::from_expr("(a 7 8 9 5)").unwrap())
    );
}

#[tokio::test]
async fn list_builders_survive_native_await_and_protected_async_errors() {
    let mut env = Env::standard();
    env.bind_native_async(
        SymbolId::from("later"),
        NativeAsyncFn {
            metadata: vec![],
            doc: String::new(),
            func: |_, args| {
                Box::new(async move {
                    tokio::task::yield_now().await;
                    if args.first() == Some(&Value::Nil) {
                        Err(Error::Runtime("async hole".into()))
                    } else {
                        Ok(Value::List(args))
                    }
                })
            },
        },
    );
    let mut fiber = Fiber::<Void, ()>::from_expr(
        "(list `(a ,@(later 1 2) ,(later 3)) (err? (try `(a ,@(later nil)))) `(ok ,4))",
        env,
        (),
    )
    .unwrap();
    assert_eq!(
        lyric::run(&mut fiber).await.unwrap(),
        Value::from_expr("((a 1 2 (3)) true (ok 4))").unwrap()
    );
}
