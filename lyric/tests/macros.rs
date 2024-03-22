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
    assert!(eval("(defn bad! () 42)").await.is_err());
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
    assert!(eval("(if false (missing!) nil)").await.is_err());
    assert!(eval("(lambda () (defmacro no () nil))").await.is_err());
}

#[tokio::test]
async fn redefinition_is_compile_time_except_deferred_eval() {
    let code = "(begin (defmacro m () 1) (defn old () (m!)) (defn deferred () (try (m!))) (defmacro m () 2) (defn fresh () (m!)) (list (old) (fresh) (deferred) (eval '(m!))))";
    assert_eq!(
        eval(code).await.unwrap(),
        Value::from_expr("(1 2 2 2)").unwrap()
    );
}

#[tokio::test]
async fn helper_snapshots_are_explicit_and_immutable() {
    let code = "(begin (for_syntax (defn helper () 1)) (defmacro m () (helper)) (for_syntax (defn helper () 2)) (def first (m!)) (defmacro m () (helper)) (list first (m!)))";
    assert_eq!(
        eval(code).await.unwrap(),
        Value::from_expr("(1 2)").unwrap()
    );
    assert!(eval("(begin (def local 1) (defmacro m () local) (m!))")
        .await
        .is_err());
    assert!(eval(
        "(begin (for_syntax (def n 0) (defn bump () (set n (+ n 1)))) (defmacro m () (bump)) (m!))"
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("captured phase binding"));
}

#[tokio::test]
async fn failed_redefinition_is_atomic_and_errors_are_catchable() {
    assert_eq!(eval("(begin (defmacro m () 42) (try (defmacro m (x x) nil)) (list (m!) (err? (try (missing!)))))").await.unwrap(),Value::from_expr("(42 true)").unwrap());
}

#[tokio::test]
async fn validation_and_phase_effect_guards() {
    for source in [
        "(begin (defmacro m () (lambda () 1)) (m!))",
        "(begin (defmacro m () (yield 42)) (m!))",
        "(begin (defmacro m () (dbg 42)) (m!))",
        "(begin (defmacro m (x & xs more) x) (m! 1))",
        "(begin (defmacro m (x) x) (m!))",
        "(begin (defmacro m (x) x) (m! 1 2))",
    ] {
        assert!(eval(source).await.is_err(), "{source}");
    }
}

#[tokio::test]
async fn phase_environment_excludes_runtime_capabilities() {
    for source in [
        "(begin (def host dbg) (defmacro m () (apply host '(42))) (m!))",
        "(begin (defmacro m () (eval '(dbg 42))) (m!))",
        "(for_syntax (def printer dbg))",
    ] {
        let error = eval(source).await.unwrap_err().to_string();
        assert!(error.contains("Undefined symbol"), "{source}: {error}");
    }
    let error = eval("(begin (defmacro identity (x) x) (macroexpand_1 (list 'identity! dbg)))")
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("expected source data"), "{error}");
}

#[tokio::test]
async fn phase_native_aliases_and_compiler_metadata_work() {
    let source = "(begin
      (for_syntax
        (def mapper map)
        (defn twice (x) (interactive :number) (+ x x)))
      (defmacro doubled (& values) `(quote ,(apply mapper (list values twice))))
      (doubled! 1 2 3))";
    assert_eq!(
        eval(source).await.unwrap(),
        Value::from_expr("(2 4 6)").unwrap()
    );
}

#[tokio::test]
async fn phase_native_allocation_limits_apply_through_aliases() {
    let grow_list = "(set xs (concat xs xs)) ".repeat(17);
    let source = format!(
        "(begin (defmacro m () (def xs '(0)) {grow_list}
          (apply map (list xs (fn (x) x)))) (m!))"
    );
    let error = eval(&source).await.unwrap_err().to_string();
    assert!(error.contains("phase map size limit"), "{error}");

    let grow_string = "(set sep (str sep sep)) ".repeat(19);
    let source = format!(
        "(begin (defmacro m () (def sep \"x\") {grow_string}
          (apply join (list sep \"\" \"\"))) (m!))"
    );
    let error = eval(&source).await.unwrap_err().to_string();
    assert!(error.contains("phase join size limit"), "{error}");
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
