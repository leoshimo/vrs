use anyhow::{bail, Context, Result};
use lyric::Form;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct Item {
    pub title: String,
    pub subtitle: Option<String>,
    pub subtitle_spans: Vec<TextSpan>,
    pub aside: Option<String>,
    pub actions: Vec<ItemCommand>,
    pub on_click: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub matched: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ItemCommand {
    pub title: String,
    pub on_click: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Page {
    pub title: String,
    pub prompt: String,
    pub get_items: String,
    pub args: String,
    pub debounce_ms: u32,
    pub on_cancel: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Close,
    PushPage { page: Page },
}

fn field<'a>(values: &'a [Form], name: &str) -> Option<&'a Form> {
    values
        .windows(2)
        .find(|pair| pair[0] == Form::keyword(name))
        .map(|pair| &pair[1])
}

fn quoted(value: Form) -> Form {
    Form::List(vec![Form::symbol("quote"), value])
}

pub fn service_request(name: &str, args: Vec<Form>) -> Form {
    let mut call = vec![Form::symbol(name)];
    call.extend(args);
    Form::List(vec![
        Form::symbol("begin"),
        Form::List(vec![Form::symbol("bind_srv"), Form::keyword("vrsjmp")]),
        Form::List(call),
    ])
}

pub fn query_request(callback: &str, args: &str, query: &str) -> Result<Form> {
    let callback = Form::from_expr(callback)?;
    if !matches!(callback, Form::Symbol(_)) {
        bail!("Page callback must be a symbol");
    }
    let args = Form::from_expr(args)?;
    if !matches!(args, Form::List(_)) {
        bail!("Page arguments must be a list");
    }
    Ok(service_request(
        "get_items",
        vec![quoted(callback), quoted(args), Form::string(query)],
    ))
}

pub fn action_request(item: &str) -> Result<Form> {
    let item = Form::from_expr(item)?;
    if !matches!(item, Form::List(_)) {
        bail!("Expected an item");
    }
    Ok(service_request("on_click", vec![quoted(item)]))
}

pub fn items(value: Form) -> Result<Vec<Item>> {
    let Form::List(items) = value else {
        bail!("Expected an item list, got {value}");
    };
    items
        .into_iter()
        .map(|item| {
            let Form::List(ref values) = item else {
                bail!("Expected an item record");
            };
            let command = item_command(&item)?;
            let actions = match field(values, "actions") {
                None => vec![],
                Some(Form::List(commands)) => {
                    commands.iter().map(item_command).collect::<Result<_>>()?
                }
                _ => bail!("Item actions must be a list"),
            };
            let (subtitle, subtitle_spans) = subtitle(values)?;
            Ok(Item {
                title: command.title,
                subtitle,
                subtitle_spans,
                aside: optional_text(values, "aside")?,
                actions,
                on_click: command.on_click,
            })
        })
        .collect()
}

// Existing strings still work. Rich subtitles are lists of strings and
// (:match "text") spans, rendered only as text nodes by the client.
fn subtitle(values: &[Form]) -> Result<(Option<String>, Vec<TextSpan>)> {
    let Some(Form::List(parts)) = field(values, "subtitle") else {
        return Ok((optional_text(values, "subtitle")?, vec![]));
    };
    let mut spans = vec![];
    let mut plain = String::new();
    for part in parts {
        let (text, matched) = match part {
            Form::String(text) => (text, false),
            Form::List(pair) => match pair.as_slice() {
                [Form::Keyword(tag), Form::String(text)] if tag.as_str() == "match" => (text, true),
                _ => bail!("Expected (:match TEXT) in subtitle"),
            },
            _ => bail!("Expected text in subtitle"),
        };
        plain.push_str(text);
        spans.push(TextSpan {
            text: text.clone(),
            matched,
        });
    }
    Ok((Some(plain), spans))
}

fn optional_text(values: &[Form], name: &str) -> Result<Option<String>> {
    match field(values, name) {
        None | Some(Form::Nil) => Ok(None),
        Some(Form::String(text)) => Ok(Some(text.clone())),
        _ => bail!("{name} must be text"),
    }
}

fn item_command(item: &Form) -> Result<ItemCommand> {
    let Form::List(values) = item else {
        bail!("Expected an action record");
    };
    let Some(Form::String(title)) = field(values, "title") else {
        bail!("Item is missing a title");
    };
    field(values, "on_click").context("Item is missing an action")?;
    Ok(ItemCommand {
        title: title.clone(),
        on_click: item.to_string(),
    })
}

pub fn action(value: Form) -> Result<Action> {
    if value == Form::keyword("close") {
        return Ok(Action::Close);
    }
    let Form::List(ref values) = value else {
        bail!("Unexpected action response: {value}");
    };
    if values.first() != Some(&Form::keyword("push_page")) {
        bail!("Unexpected action response: {value}");
    }
    let Some(Form::Symbol(callback)) = field(values, "get_items") else {
        bail!("Page is missing its callback");
    };
    let Some(Form::String(prompt)) = field(values, "prompt") else {
        bail!("Page is missing its prompt");
    };
    let args = match field(values, "args") {
        None => "()".to_string(),
        Some(args @ Form::List(_)) => args.to_string(),
        _ => bail!("Page arguments must be a list"),
    };
    let debounce_ms = match field(values, "debounce_ms") {
        None => 0,
        Some(Form::Int(ms)) if *ms >= 0 && *ms <= 60_000 => *ms as u32,
        _ => bail!("Page debounce must be between 0 and 60000 milliseconds"),
    };
    Ok(Action::PushPage {
        page: Page {
            title: optional_text(values, "title")?.unwrap_or_else(|| prompt.clone()),
            prompt: prompt.clone(),
            get_items: callback.as_str().into(),
            args,
            debounce_ms,
            on_cancel: field(values, "on_cancel")
                .filter(|value| **value != Form::Nil)
                .map(|command| {
                    Form::List(vec![Form::keyword("on_click"), command.clone()]).to_string()
                }),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_text_and_arguments_are_data() {
        let text = "quotes \" \\ newline\n) (exec \"unexpected\")";
        let request =
            query_request("call_items", "(focus_window ((:os/window :id 7)))", text).unwrap();
        assert_eq!(Form::from_expr(&request.to_string()).unwrap(), request);
        let Form::List(body) = request else { panic!() };
        let Form::List(call) = &body[2] else { panic!() };
        assert_eq!(call[3], Form::string(text));
        assert!(query_request("(exec)", "()", text).is_err());
        assert!(query_request("root_items", "oops", text).is_err());
    }
    #[test]
    fn page_and_item_validation() {
        let page = Form::from_expr(
            "(:push_page :prompt \"Read Later\" :get_items read_later_items :debounce_ms 200)",
        )
        .unwrap();
        let Action::PushPage { page } = action(page).unwrap() else {
            panic!()
        };
        assert_eq!(page.args, "()");
        assert_eq!(page.title, "Read Later");
        assert_eq!(page.debounce_ms, 200);
        assert!(action(
            Form::from_expr("(:push_page :prompt \"x\" :get_items f :debounce_ms -1)").unwrap()
        )
        .is_err());
        assert_eq!(action(Form::keyword("close")).unwrap(), Action::Close);
        assert!(items(Form::from_expr("((:title 42))").unwrap()).is_err());
        assert!(action(Form::keyword("unexpected")).is_err());
        let rows = items(
            Form::from_expr(
                r#"((:title "Article" :subtitle "example.test" :aside "Saved today"
            :on_click (open_url "https://example.test")
            :actions ((:title "Copy URL" :on_click (set_clipboard "https://example.test")))))"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(rows[0].subtitle.as_deref(), Some("example.test"));
        assert!(rows[0].subtitle_spans.is_empty());
        assert_eq!(rows[0].actions[0].title, "Copy URL");
        assert!(action_request(&rows[0].actions[0].on_click).is_ok());
        assert!(
            items(Form::from_expr("((:title \"x\" :on_click (x) :actions (42)))").unwrap())
                .is_err()
        );
        let rows = items(
            Form::from_expr(
                r#"((:title "Note" :on_click (open_note)
          :subtitle ("<script>" (:match "needle") "</script>")))"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(rows[0].subtitle.as_deref(), Some("<script>needle</script>"));
        assert_eq!(
            rows[0].subtitle_spans[1],
            TextSpan {
                text: "needle".into(),
                matched: true
            }
        );
        assert!(!rows[0].subtitle_spans[0].matched);
        assert!(items(
            Form::from_expr(
                r#"((:title "Note" :on_click (open_note)
          :subtitle ((:match 42))))"#
            )
            .unwrap()
        )
        .is_err());
    }
}
