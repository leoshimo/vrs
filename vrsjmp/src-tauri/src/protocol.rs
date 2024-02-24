use anyhow::{bail, Context, Result};
use lyric::Form;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct Item {
    pub title: String,
    pub on_click: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Page {
    pub prompt: String,
    pub get_items: String,
    pub args: String,
    pub debounce_ms: u32,
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
            let Some(Form::String(title)) = field(values, "title") else {
                bail!("Item is missing a title");
            };
            field(values, "on_click").context("Item is missing an action")?;
            Ok(Item {
                title: title.clone(),
                on_click: item.to_string(),
            })
        })
        .collect()
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
            prompt: prompt.clone(),
            get_items: callback.as_str().into(),
            args,
            debounce_ms,
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
        assert_eq!(page.debounce_ms, 200);
        assert!(action(
            Form::from_expr("(:push_page :prompt \"x\" :get_items f :debounce_ms -1)").unwrap()
        )
        .is_err());
        assert_eq!(action(Form::keyword("close")).unwrap(), Action::Close);
        assert!(items(Form::from_expr("((:title 42))").unwrap()).is_err());
        assert!(action(Form::keyword("unexpected")).is_err());
    }
}
