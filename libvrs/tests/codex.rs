use std::{fs, process::Command};
use vrs::{ProcessResult, Program, Runtime, Val};

fn definitions(source: &str, names: Option<&[&str]>) -> Vec<Val> {
    lyric::parse_script(source)
        .unwrap()
        .into_iter()
        .filter(|form| match form {
            lyric::Form::List(values) if values.first() == Some(&lyric::Form::symbol("defn!")) => {
                names.is_none_or(|names| {
                    names
                        .iter()
                        .any(|name| values.get(1) == Some(&lyric::Form::symbol(name)))
                })
            }
            _ => false,
        })
        .map(Val::from)
        .collect()
}

async fn evaluate(codex_home: &std::path::Path, script: &str) -> Val {
    // Use the real shell/SQLite reader against an isolated fixture, not the app.
    let env = serde_json::to_string(&format!("CODEX_HOME={}", codex_home.display())).unwrap();
    let mut body = vec![Val::symbol("begin")];
    body.extend(definitions(include_str!("../../scripts/codex.ll"), None));
    body.extend(definitions(
        include_str!("../../scripts/vrsjmp.ll"),
        Some(&["make_item", "push_page", "codex_page", "codex_items"]),
    ));
    body.extend(
        lyric::parse_script(&format!(
            r#"
            (def system_exec exec)
            (defn! exec (program flags mode option input)
              (system_exec "env" {env} program flags mode option input))
            (def codex_cache nil)
            {script}
            "#
        ))
        .unwrap()
        .into_iter()
        .map(Val::from),
    );
    let runtime = Runtime::new("test");
    let result = runtime
        .run(Program::from_val(Val::List(body)).unwrap())
        .await
        .unwrap()
        .join()
        .await
        .unwrap();
    let ProcessResult::Done(value) = result.status.unwrap() else {
        panic!("script did not complete")
    };
    value
}

#[tokio::test]
async fn recent_threads_search_unread_and_open() {
    let dir = std::env::temp_dir().join(format!(
        "vrs-codex-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("sqlite")).unwrap();
    let catalog = dir.join("sqlite/codex-dev.db");
    let sql = r#"
        CREATE TABLE local_thread_catalog (
          thread_id TEXT, display_title TEXT, host_id TEXT, cwd TEXT,
          source_recency_at REAL, source_updated_at REAL, source_created_at REAL,
          missing_candidate INTEGER, source_kind TEXT);
        INSERT INTO local_thread_catalog VALUES
          ('old', 'Earlier task', 'local', NULL, 0, 100, 100, 0, 'cli'),
          ('middle id', 'D''Artagnan "notes" 東京', 'local', '/work/vrs', 200, 200, 200, 0, 'vscode'),
          ('new', 'Build adapter', 'remote-ssh-discovered:node', '/work/adapter', 300, 300, 300, 0, 'vscode'),
          ('missing', 'Removed', 'local', NULL, 900, 900, 900, 1, 'vscode'),
          ('chat', 'ChatGPT chat', 'chatgpt:account', NULL, 800, 800, 800, 0, 'chatgpt'),
          ('agent', 'Subagent', 'local', NULL, 700, 700, 700, 0, 'subAgent');
    "#;
    assert!(Command::new("sqlite3")
        .arg(&catalog)
        .arg(sql)
        .status()
        .unwrap()
        .success());
    let original = fs::read(&catalog).unwrap();
    let state = dir.join(".codex-global-state.json");
    fs::write(
        &state,
        r#"{"electron-persisted-atom-state":{"unread-thread-ids-by-host-v1":{
      "local":["old"], "remote-ssh-discovered:node":["new", "middle id"]}}}"#,
    )
    .unwrap();

    let value = evaluate(
        &dir,
        r#"
      (def rows (codex_items ""))
      (defn! exec (program flags mode option input) (error "Search must use the cache"))
      (def matches (list
        (map (codex_items "東京") (fn (row) (get row :title)))
        (map (codex_items "/work/vrs") (fn (row) (get row :title)))
        (map (codex_items "node") (fn (row) (get row :title)))
        (codex_items "'; DROP TABLE local_thread_catalog; --")))
      (def opened nil)
      (defn! exec (program url) (set opened (list program url)) '(:exit 0))
      (eval (get (get rows 1) :on_click))
      (list (map rows (fn (row) (get row :title)))
            (map rows (fn (row) (contains? (get row :aside) "Unread")))
            (map rows (fn (row) (get row :actions)))
            matches opened (get (codex_page) :get_items) codex_cache)
    "#,
    )
    .await;
    assert_eq!(
        value,
        Val::from_expr(
            r#"
      (("Build adapter" "D'Artagnan \"notes\" 東京" "Earlier task")
       (true false true) (nil nil nil)
       (("D'Artagnan \"notes\" 東京") ("D'Artagnan \"notes\" 東京") ("Build adapter") ())
       ("open" "codex://threads/middle%20id") codex_items nil)
    "#
        )
        .unwrap()
    );
    assert_eq!(fs::read(&catalog).unwrap(), original);

    fs::write(&state, "invalid JSON").unwrap();
    assert_eq!(
        evaluate(&dir, "(err? (try (get_codex_threads)))").await,
        Val::Bool(true)
    );
    fs::remove_file(&state).unwrap();
    assert_eq!(
        evaluate(
            &dir,
            "(map (get_codex_threads) (fn (thread) (get thread :unread)))"
        )
        .await,
        Val::from_expr("(false false false)").unwrap()
    );
    assert!(Command::new("sqlite3")
        .arg(&catalog)
        .arg("DELETE FROM local_thread_catalog")
        .status()
        .unwrap()
        .success());
    // Also exercise the release catalog name and SQLite's empty stdout case.
    let release_catalog = dir.join("sqlite/codex.db");
    fs::rename(&catalog, &release_catalog).unwrap();
    assert_eq!(
        evaluate(&dir, "(get_codex_threads)").await,
        Val::List(vec![])
    );
    fs::remove_file(&release_catalog).unwrap();
    assert_eq!(
        evaluate(&dir, "(err? (try (get_codex_threads)))").await,
        Val::Bool(true)
    );
    assert!(!release_catalog.exists());
    fs::remove_dir_all(dir).unwrap();
}
