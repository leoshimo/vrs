use lyric::{Env, Fiber, Val};
use void::Void;
type Value = Val<Void, ()>;

async fn eval(source: &str) -> lyric::Result<Value> {
    let mut fiber = Fiber::from_expr(source, Env::standard(), ())?;
    lyric::run(&mut fiber).await
}

#[tokio::test]
async fn raw_arguments_and_inspection_never_execute_output() {
    assert_eq!(eval("(begin (defmacro discard (& ignored) nil) (discard! (missing!)) (macroexpand_1 '(when! true (missing))))").await.unwrap(), Value::from_expr("(if true (begin (missing)) nil)").unwrap());
    assert_eq!(
        eval("(begin (def n 0) (macroexpand_1 '(when! true (set n 1))) n)")
            .await
            .unwrap(),
        Value::Int(0)
    );
    assert_eq!(eval("(when! false (missing))").await.unwrap(), Value::Nil);
}

#[tokio::test]
async fn explicit_marker_and_outer_expansion_contract() {
    let code = "(begin (defmacro twice (x) `(when! true ,x ,x)) (list (macroexpand_1 '(twice! (+ 1 2))) (macroexpand '(twice! (+ 1 2))) (macroexpand '(begin (when! true 42))) (macroexpand_1 ''(when! true 42))))";
    assert_eq!(eval(code).await.unwrap(),Value::from_expr("((when! true (+ 1 2) (+ 1 2)) (if true (begin (+ 1 2) (+ 1 2)) nil) (begin (when! true 42)) (quote (when! true 42)))").unwrap());
    assert!(eval("(when true 42)").await.is_err());
    assert!(eval("(missing! 42)")
        .await
        .unwrap_err()
        .to_string()
        .contains("undefined macro missing!"));
    assert!(eval("(defn! bad! () 42)").await.is_err());
}

#[tokio::test]
async fn nested_expression_positions_and_templates() {
    assert_eq!(eval("(begin (defmacro add (a b) `(+ ,a ,b)) (let ((x (add! 1 2))) (match x (3 `(literal (add! 8 9) ,(add! x 4))))))").await.unwrap(),Value::from_expr("(literal (add! 8 9) 7)").unwrap());
    assert_eq!(
        eval("(begin (defmacro no () (error \"wrong phase\")) `(quote (no!)))")
            .await
            .unwrap(),
        Value::from_expr("(quote (no!))").unwrap()
    );
    assert_eq!(eval("(if false (missing!) nil)").await.unwrap(), Value::Nil);
    assert!(eval("(lambda () (defmacro no () nil))").await.is_ok());
}

#[tokio::test]
async fn redefinition_affects_existing_functions_at_the_next_call() {
    let code = "(begin (defmacro m () 1) (defn! old () (m!)) (defn! deferred () (try (m!))) (defmacro m () 2) (defn! fresh () (m!)) (list (old) (fresh) (deferred) (eval '(m!))))";
    assert_eq!(
        eval(code).await.unwrap(),
        Value::from_expr("(2 2 2 2)").unwrap()
    );
}

#[tokio::test]
async fn ordinary_helpers_and_captured_state_are_available() {
    let code = "(begin (defn! helper () 1) (defmacro m () (helper))
      (defn! helper () 2) (def first (m!)) (list first (m!)))";
    assert_eq!(
        eval(code).await.unwrap(),
        Value::from_expr("(2 2)").unwrap()
    );
    assert_eq!(
        eval("(begin (def local 1) (defmacro m () local) (m!))")
            .await
            .unwrap(),
        Value::Int(1)
    );
    // The old wrapper remains compatible, but no longer creates a separate phase.
    assert_eq!(
        eval(
            "(begin (for_syntax (def n 0) (defn! bump () (set n (+ n 1))))
      (defmacro m () (bump)) (list (m!) n))"
        )
        .await
        .unwrap(),
        Value::from_expr("(1 1)").unwrap()
    );
}

#[tokio::test]
async fn expansion_runs_at_each_executed_call_and_can_inspect_caller_locals() {
    let source = "(begin
      (def expansions 0)
      (def name 10)
      (defn! helper () name)
      (defmacro inspect (expr)
        (set expansions (+ expansions 1))
        (list 'quote (list (helper) (eval_caller expr))))
      (defn! run (name) (inspect! name))
      (list expansions (run 20) (run 30) expansions))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("(0 (10 20) (10 30) 2)").unwrap()
    );
    assert!(eval("(eval_caller 'name)")
        .await
        .unwrap_err()
        .to_string()
        .contains("only available during macro expansion"));
}

#[tokio::test]
async fn failed_redefinition_is_atomic_and_errors_are_catchable() {
    assert_eq!(eval("(begin (defmacro m () 42) (try (defmacro m (x x) nil)) (list (m!) (err? (try (missing!)))))").await.unwrap(),Value::from_expr("(42 true)").unwrap());
}

#[tokio::test]
async fn macro_signature_and_source_result_validation() {
    for source in [
        "(begin (defmacro m () (lambda () 1)) (m!))",
        "(begin (defmacro m (x & xs more) x) (m! 1))",
        "(begin (defmacro m (x) x) (m!))",
        "(begin (defmacro m (x) x) (m! 1 2))",
    ] {
        assert!(eval(source).await.is_err(), "{source}");
    }
}

#[tokio::test]
async fn runtime_effects_work_but_macro_inputs_remain_source_data() {
    assert_eq!(
        eval(
            "(begin (def n 0) (defn! effect () (set n (+ n 1)))
      (defmacro m () (effect) '(set n 99))
      (macroexpand_1 '(m!)) n)"
        )
        .await
        .unwrap(),
        Value::Int(1)
    );
    let error = eval("(begin (defmacro identity (x) x) (macroexpand_1 (list 'identity! dbg)))")
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("expected source data"), "{error}");
}

#[test]
fn transformers_suspend_and_resume_on_the_calling_fiber() {
    let mut fiber: Fiber<Void, ()> = Fiber::from_expr(
        "(begin
      (defmacro m (expression) (yield :expanding) expression)
      (let ((x 42)) (m! x)))",
        Env::standard(),
        (),
    )
    .unwrap();
    assert_eq!(
        fiber.start().unwrap(),
        lyric::Signal::Yield(Value::keyword("expanding"))
    );
    assert_eq!(
        fiber.resume(Ok(Value::Nil)).unwrap(),
        lyric::Signal::Done(Value::Int(42))
    );
}

#[tokio::test]
async fn expansion_native_aliases_and_compiler_metadata_work() {
    let source = "(begin
      (for_syntax
        (def mapper map)
        (defn! twice (x) (interactive :number) (+ x x)))
      (defmacro doubled (& values) `(quote ,(apply mapper (list values twice))))
      (doubled! 1 2 3))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("(2 4 6)").unwrap()
    );
}

#[tokio::test]
async fn expansion_native_allocation_limits_apply_through_aliases() {
    let grow_list = "(set xs (concat xs xs)) ".repeat(17);
    let source = format!(
        "(begin (defmacro m () (def xs '(0)) {grow_list}
          (apply map (list xs (fn (x) x)))) (m!))"
    );
    let error = eval(&source).await.unwrap_err().to_string();
    assert!(error.contains("expansion map size limit"), "{error}");

    let grow_string = "(set sep (str sep sep)) ".repeat(19);
    let source = format!(
        "(begin (defmacro m () (def sep \"x\") {grow_string}
          (apply join (list sep \"\" \"\"))) (m!))"
    );
    let error = eval(&source).await.unwrap_err().to_string();
    assert!(error.contains("expansion join size limit"), "{error}");
}

#[tokio::test]
async fn gensym_gives_single_evaluation_and_source_round_trip() {
    let code = "(begin (defmacro or_else (value fallback) (def temp (gensym \"value\")) `(let ((,temp ,value)) (if ,temp ,temp ,fallback))) (def n 0) (def expansion (macroexpand_1 '(or_else! (begin (set n (+ n 1)) n) (set n 99)))) (list (eval expansion) n (eq? expansion (read (pretty expansion))) (eq? (gensym) (gensym))))";
    assert_eq!(
        eval(code).await.unwrap(),
        Value::from_expr("(1 1 true false)").unwrap()
    );
}

#[tokio::test]
async fn gensym_names_put_readable_hints_first_and_remain_source_symbols() {
    let mut names = std::collections::HashSet::new();
    for (argument, prefix) in [
        ("", "tmp"),
        ("\"value\"", "value"),
        ("\"value\"", "value"),
        ("\"\"", "tmp"),
        ("\"!?雪\"", "tmp"),
        ("\"42\"", "_42"),
        ("\"service-name\"", "servicename"),
        ("\"true\"", "true"),
        (
            "\"abcdefghijklmnopqrstuvwxyz0123456789\"",
            "abcdefghijklmnopqrstuvwxyz012345",
        ),
    ] {
        let symbol = eval(&format!("(gensym {argument})")).await.unwrap();
        let name = symbol.as_symbol().unwrap().to_string();
        let suffix = name.strip_prefix(&format!("{prefix}__")).unwrap();
        assert_eq!(suffix.len(), 16);
        assert!(names.insert(name.clone()));
        assert_eq!(Value::from_expr(&name).unwrap(), symbol);
        assert_eq!(
            eval(&format!("(let (({name} 42)) {name})")).await.unwrap(),
            Value::Int(42)
        );
    }
}

#[tokio::test]
async fn expansion_limits_recover() {
    assert_eq!(eval("(begin (defmacro forever () '(forever!)) (list (err? (try (forever!))) (when! true 42)))").await.unwrap(), Value::from_expr("(true 42)").unwrap());
    assert!(eval("(begin (defmacro spin () (loop nil)) (spin!))")
        .await
        .unwrap_err()
        .to_string()
        .contains("instruction limit"));
    assert_eq!(eval("(begin (defmacro recur () '(begin (recur!))) (list (err? (try (recur!))) (when! true 42)))").await.unwrap(), Value::from_expr("(true 42)").unwrap());
    assert!(
        eval("(begin (defmacro recur () (macroexpand_1 '(recur!))) (recur!))")
            .await
            .unwrap_err()
            .to_string()
            .contains("depth exceeded")
    );
}

#[tokio::test]
async fn generated_definitions_are_inert_during_inspection() {
    let source = "(begin (defmacro maker () '(defmacro made () 42))
      (macroexpand_1 '(maker!)) (def absent (err? (try (made!))))
      (maker!) (list absent (made!)))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("(true 42)").unwrap()
    );
}

#[tokio::test]
async fn intermediate_values_and_reader_nesting_are_bounded() {
    assert!(
        eval("(begin (defmacro grow () (def s \"x\") (loop (set s (str s s)))) (grow!))")
            .await
            .unwrap_err()
            .to_string()
            .contains("string size limit")
    );
    let source = format!("{}0{}", "(".repeat(300), ")".repeat(300));
    assert!(lyric::parse(&source)
        .unwrap_err()
        .to_string()
        .contains("nesting"));
}

#[tokio::test]
async fn boolean_macros_short_circuit_and_preserve_operand_values() {
    for (source, expected) in [
        ("(and!)", "true"),
        ("(or!)", "nil"),
        ("(and! :last)", ":last"),
        ("(or! :last)", ":last"),
        ("(and! true 3 \"yes\" '(last))", "(last)"),
        ("(or! nil false 0 \"\" '() :last)", ":last"),
        ("(or! false 0 \"\" '())", "()"),
        ("(and! false (missing!))", "false"),
        ("(or! true (missing!))", "true"),
    ] {
        assert_eq!(
            eval(source).await.unwrap(),
            Value::from_expr(expected).unwrap(),
            "{source}"
        );
    }
    for falsy in ["nil", "false", "0", "\"\"", "'()"] {
        assert_eq!(
            eval(&format!("(and! true {falsy} (error \"unreachable\"))"))
                .await
                .unwrap(),
            eval(falsy).await.unwrap()
        );
    }
    for name in ["and!", "or!"] {
        assert!(eval(&format!("({name} :invalid-condition 42)"))
            .await
            .is_err());
    }
}

#[tokio::test]
async fn boolean_macros_evaluate_once_in_order_without_capturing_names() {
    let source = "(begin
      (def trace '())
      (defn! visit (value) (set trace (push trace value)) value)
      (def value 42)
      (def a (and! (visit 1) (visit 2) (visit 0) (visit 99)))
      (def b (or! (visit false) (visit 3) (visit 99)))
      (def c (or! false value))
      (def d (and! true value))
      (def expansion (macroexpand_1 '(or! (visit 5) (visit 6))))
      (list a b c d trace (eval expansion) trace))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("(0 3 42 42 (1 2 0 false 3) 5 (1 2 0 false 3 5))").unwrap()
    );
}

#[tokio::test]
async fn defn_expansion_does_not_define_the_function() {
    let expanded =
        Value::from_expr("(def echo (fn (x) \"Echo\" (interactive :number) x))").unwrap();
    for expand in ["macroexpand_1", "macroexpand"] {
        let source = format!(
            "(begin
          (def form ({expand} '(defn! echo (x) \"Echo\" (interactive :number) x)))
          (list form (err? (try echo))))"
        );
        assert_eq!(
            eval(&source).await.unwrap(),
            Value::List(vec![expanded.clone(), Value::Bool(true)])
        );
    }
    let source = "(begin
      (defmacro named () '(defn! echo (x) x))
      (list (macroexpand_1 '(named!)) (macroexpand '(named!))
            (macroexpand ''(defn! echo (x) x))))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("((defn! echo (x) x) (def echo (fn (x) x)) (quote (defn! echo (x) x)))")
            .unwrap()
    );
}

#[tokio::test]
async fn defn_retains_lexical_state_recursion_and_function_metadata() {
    let source = "(begin
      (defn! make_counter (value)
        (defn! next (amount) \"Advance\" (interactive :number)
          (set value (+ value amount)) value)
        next)
      (def counter (make_counter 40))
      (defn! count (n) (if (eq? n 0) 0 (+ 1 (count (- n 1)))))
      (list (counter 1) (counter 2) (count 3)
        (get (meta counter) :doc) (get (meta counter) :interactive)
        (get (meta counter) :args)))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("(41 43 3 \"Advance\" true ((:type :number :name amount)))").unwrap()
    );
}

#[tokio::test]
async fn defn_uses_runtime_macro_redefinitions_including_existing_call_sites() {
    let source = "(begin
      (defn! before () 1)
      (defn! create () (defn! later () 2) (later))
      (defmacro defn (name params & body) `(def ,name (fn ,params 42)))
      (defn! after () 3)
      (list (before) (create) (after)
        (macroexpand_1 '(defn! inspected () 4))))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("(1 42 42 (def inspected (fn () 42)))").unwrap()
    );
}

#[tokio::test]
async fn defn_errors_are_catchable_when_the_definition_executes() {
    let source = "(begin
      (defn! delayed () (defn! broken))
      (list (lambda? delayed) (err? (try (delayed)))
        (err? (try (defn! missing_body ())))
        (err? (try (defn! invalid_params 42 1)))
        (err? (try (defn! invalid_metadata (x) (interactive) x)))
        (begin (defn! recovered () 42) (recovered))))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("(true true true true true 42)").unwrap()
    );
    assert_eq!(
        eval("(if false (defn! broken) :skipped)").await.unwrap(),
        Value::keyword("skipped")
    );
}

#[tokio::test]
async fn defn_requires_the_macro_marker() {
    let unmarked = Value::from_expr("(defn echo (x) x)").unwrap();
    for expand in ["macroexpand_1", "macroexpand"] {
        assert_eq!(
            eval(&format!("({expand} '(defn echo (x) x))"))
                .await
                .unwrap(),
            unmarked
        );
    }
    assert!(matches!(eval("(defn echo (x) x)").await,
        Err(lyric::Error::UndefinedSymbol(name)) if name.as_str() == "defn"));
}
