use std::{path::Path, sync::Arc, time::Duration};
use vrs::{Client, Connection, Form, KeywordId, Program, Runtime};

async fn fixture(path: &Path) -> (Runtime, Arc<Client>) {
    let runtime = Runtime::new("palette-config-test");
    let definitions = lyric::parse_script(include_str!("../../scripts/vrsjmp.ll"))
        .unwrap().into_iter().filter(|form| matches!(form, Form::List(values)
            if values.first() == Some(&Form::symbol("defn!"))
                && matches!(values.get(1), Some(Form::Symbol(name))
                    if ["validate_ui_config", "get_ui_config", "set_ui_config", "appearance_items", "appearance_page", "push_page"].contains(&name.as_str()))))
        .map(|form| form.to_string()).collect::<Vec<_>>().join("\n");
    let source = format!(
        r#"
        (def ui_config_path {})
        (def ui_config '(:theme :neutral :appearance :system))
        {definitions}
        (spawn_srv! :vrsjmp :interface '(get_ui_config set_ui_config appearance_items appearance_page))
    "#,
        Form::string(path.to_str().unwrap())
    );
    runtime
        .run(Program::from_script(&source).unwrap())
        .await
        .unwrap()
        .join()
        .await
        .unwrap()
        .status
        .unwrap();
    let (local, remote) = Connection::pair().unwrap();
    runtime.handle_conn(remote).await.unwrap();
    (runtime, Arc::new(Client::new(local)))
}

async fn evaluate(client: &Client, source: &str) -> Result<Form, String> {
    tokio::time::timeout(
        Duration::from_secs(3),
        client.request(Form::from_expr(source).unwrap()),
    )
    .await
    .unwrap()
    .unwrap()
    .contents
    .map_err(|error| error.to_string())
}

#[tokio::test]
async fn configuration_persists_before_notifying_and_rejects_invalid_updates() {
    let path = std::env::temp_dir().join(format!(
        "vrsjmp-config-{}-{}.ll",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let (_runtime, client) = fixture(&path).await;
    let mut events = client.subscribe(KeywordId::from("vrsjmp")).await.unwrap();
    assert_eq!(
        evaluate(&client, "(begin (bind_srv :vrsjmp) (get_ui_config))")
            .await
            .unwrap(),
        Form::from_expr("(:theme :neutral :appearance :system)").unwrap()
    );
    let updated = evaluate(
        &client,
        "(begin (bind_srv :vrsjmp) (set_ui_config '(:theme :warm :appearance :dark)))",
    )
    .await
    .unwrap();
    assert_eq!(
        events.recv().await.unwrap(),
        Form::keyword("config_changed")
    );
    assert_eq!(
        Form::from_expr(&std::fs::read_to_string(&path).unwrap()).unwrap(),
        updated
    );
    assert!(evaluate(
        &client,
        "(begin (bind_srv :vrsjmp) (set_ui_config '(:theme :unknown)))"
    )
    .await
    .is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(40), events.recv())
            .await
            .is_err()
    );
    assert_eq!(
        Form::from_expr(&std::fs::read_to_string(&path).unwrap()).unwrap(),
        updated
    );
    let (_restarted, reconnected) = fixture(&path).await;
    assert_eq!(
        evaluate(&reconnected, "(begin (bind_srv :vrsjmp) (get_ui_config))")
            .await
            .unwrap(),
        updated
    );
    assert_eq!(
        evaluate(
            &reconnected,
            "(begin (bind_srv :vrsjmp) (set_ui_config '(:theme :cool)))"
        )
        .await
        .unwrap(),
        Form::from_expr("(:theme :cool :appearance :dark)").unwrap()
    );
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn appearance_menu_updates_config_and_keeps_the_page_open() {
    let path = std::env::temp_dir().join(format!("vrsjmp-menu-{}.ll", std::process::id()));
    let (_runtime, client) = fixture(&path).await;
    assert_eq!(evaluate(&client, "(begin (bind_srv :vrsjmp) (appearance_page))").await.unwrap(),
        Form::from_expr("(:push_page :get_items appearance_items :prompt \"Search appearance…\" :title \"Palette Appearance\")").unwrap());
    for (query, key, expected) in [
        ("Warm", ":theme", ":warm"),
        ("Dark", ":appearance", ":dark"),
        ("Cool", ":theme", ":cool"),
        ("Light", ":appearance", ":light"),
        ("System", ":appearance", ":system"),
        ("Neutral", ":theme", ":neutral"),
    ] {
        let command = format!("(begin (bind_srv :vrsjmp) (eval (get (get (appearance_items \"{query}\") 0) :on_click)))");
        assert_eq!(
            evaluate(&client, &command).await.unwrap(),
            Form::keyword("refresh")
        );
        let actual = evaluate(
            &client,
            &format!("(begin (bind_srv :vrsjmp) (get (get_ui_config) {key}))"),
        )
        .await
        .unwrap();
        assert_eq!(actual, Form::from_expr(expected).unwrap());
        let selected = evaluate(
            &client,
            &format!(
                "(begin (bind_srv :vrsjmp) (get (get (appearance_items \"{query}\") 0) :aside))"
            ),
        )
        .await
        .unwrap();
        assert_eq!(selected, Form::string("Selected"));
    }
    std::fs::remove_file(path).unwrap();
}
