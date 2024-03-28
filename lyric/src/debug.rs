//! Observation boundaries in the VM. A host supplies a nonblocking sink; without
//! one, debug scopes execute normally and retain no runtime observations.
use crate::{source::SourceSite, Extern, Inst, Locals, Val};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preview {
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub parent: Option<String>,
    pub run: String,
    pub kind: String,
    pub site: SourceSite,
    pub arguments: Vec<Preview>,
    pub arguments_truncated: bool,
    pub result: Option<Preview>,
    pub status: String,
    pub elapsed_us: u64,
    pub started_ms: u64,
}

/// The sink must not wait for consumers or call back into the running fiber.
pub trait Observer: std::fmt::Debug + Send + Sync {
    fn observe(&self, event: Event);
}

#[derive(Debug)]
pub(crate) struct Trace {
    pub event: Event,
    started: Instant,
    observer: Arc<dyn Observer>,
}
impl Trace {
    pub fn start<T: Extern, L: Locals>(
        observer: Arc<dyn Observer>,
        parent: Option<&Trace>,
        kind: &str,
        mut site: SourceSite,
        args: &[Val<T, L>],
    ) -> Self {
        let id = crate::Ref::unique().0;
        site.expression = clipped(&site.expression, 4096);
        site.form = clipped(&site.form, 4096);
        site.file = clipped(&site.file, 1024);
        let event = Event {
            run: parent
                .map(|p| p.event.run.clone())
                .unwrap_or_else(|| id.clone()),
            parent: parent.map(|p| p.event.id.clone()),
            id,
            kind: kind.into(),
            site,
            arguments: args.iter().take(16).map(preview).collect(),
            arguments_truncated: args.len() > 16,
            result: None,
            status: "running".into(),
            elapsed_us: 0,
            started_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        observer.observe(event.clone());
        Self {
            event,
            started: Instant::now(),
            observer,
        }
    }
    pub fn finish(&mut self, status: &str, result: Option<Preview>) {
        if self.event.status != "running" {
            return;
        }
        self.event.status = status.into();
        self.event.result = result;
        self.event.elapsed_us = self.started.elapsed().as_micros().min(u64::MAX as u128) as u64;
        self.observer.observe(self.event.clone());
    }
}
impl Drop for Trace {
    fn drop(&mut self) {
        self.finish("cancelled", None);
    }
}

pub(crate) fn clipped(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.into();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// Bounded traversal: do not stringify a huge list or a closure environment and
/// only truncate afterward. Function values display their source when available.
pub fn preview<T: Extern, L: Locals>(v: &Val<T, L>) -> Preview {
    struct Writer {
        text: String,
        left: usize,
        truncated: bool,
    }
    impl Writer {
        fn put(&mut self, s: &str) {
            if s.len() > self.left {
                self.truncated = true;
            }
            let end = s
                .char_indices()
                .map(|(i, ch)| i + ch.len_utf8())
                .take_while(|end| *end <= self.left)
                .last()
                .unwrap_or(0);
            self.text.push_str(&s[..end]);
            self.left -= end;
        }
        fn value<T: Extern, L: Locals>(&mut self, v: &Val<T, L>, depth: usize) {
            if self.left == 0 || depth > 16 {
                self.truncated = true;
                return;
            }
            match v {
                Val::List(items) => {
                    self.put("(");
                    for (i, item) in items.iter().enumerate() {
                        if self.left == 0 {
                            self.truncated = true;
                            break;
                        }
                        if i > 0 {
                            self.put(" ");
                        }
                        self.value(item, depth + 1);
                    }
                    self.put(")");
                }
                Val::String(s) => {
                    self.put("\"");
                    for ch in s.chars() {
                        if self.left == 0 {
                            self.truncated = true;
                            break;
                        }
                        match ch {
                            '\n' => self.put("\\n"),
                            '\r' => self.put("\\r"),
                            '\t' => self.put("\\t"),
                            '"' => self.put("\\\""),
                            '\\' => self.put("\\\\"),
                            ch => self.put(ch.encode_utf8(&mut [0; 4])),
                        }
                    }
                    self.put("\"");
                }
                Val::Lambda(l) => match l.code.first() {
                    Some(Inst::FunctionSource(site)) => self.put(&site.form),
                    _ => {
                        self.put("(fn (");
                        for (i, p) in l.params.iter().enumerate() {
                            if i > 0 {
                                self.put(" ");
                            }
                            self.put(p.as_str());
                        }
                        self.put(") …)");
                    }
                },
                Val::NativeFn(_) => self.put("<native function>"),
                Val::NativeAsyncFn(_) => self.put("<async function>"),
                Val::Bytecode(_) => self.put("<bytecode>"),
                Val::Symbol(s) => self.put(s.as_str()),
                Val::Keyword(s) => {
                    self.put(":");
                    self.put(s.as_str());
                }
                _ => self.put(&v.to_string()),
            }
        }
    }
    let mut writer = Writer {
        text: String::new(),
        left: 1024,
        truncated: false,
    };
    writer.value(v, 0);
    if writer.truncated {
        writer.text.push('…');
    }
    Preview {
        text: writer.text,
        truncated: writer.truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Env, Fiber, Signal};
    use std::sync::Mutex;
    type Value = Val<void::Void, ()>;
    #[derive(Debug, Default)]
    struct Events(Mutex<Vec<Event>>);
    impl Observer for Events {
        fn observe(&self, e: Event) {
            self.0.lock().unwrap().push(e);
        }
    }
    fn fiber(source: &str) -> (Fiber<void::Void, ()>, Arc<Events>) {
        let request: Value = crate::source::request(source, "example.ll", 1, 1).into();
        let mut f = Fiber::from_val(&request, Env::standard(), ()).unwrap();
        let events = Arc::new(Events::default());
        f.set_observer(events.clone());
        (f, events)
    }

    #[tokio::test]
    async fn observes_map_callbacks_real_arguments_and_results_once() {
        let (mut f,events) = fiber("(def n 0) (defn! twice (x) (set n (+ n 1)) (+ x x))\n(dbg! (def answer (map '(2 3) twice)) answer)\n(list n answer)");
        assert_eq!(crate::run(&mut f).await.unwrap().to_string(), "(2 (4 6))");
        let events = events.0.lock().unwrap();
        let completed: Vec<_> = events.iter().filter(|e| e.status == "returned").collect();
        assert_eq!(completed.iter().filter(|e| e.kind == "scope").count(), 1);
        let scope = completed.iter().find(|e| e.kind == "scope").unwrap();
        assert!(!scope.site.generated);
        assert!(scope.site.expression.starts_with("(dbg!"));
        let map = completed
            .iter()
            .find(|e| e.site.form == "(map '(2 3) twice)")
            .unwrap();
        assert_eq!(map.arguments[0].text, "(2 3)");
        assert_eq!(map.result.as_ref().unwrap().text, "(4 6)");
        let callbacks: Vec<_> = completed.iter().filter(|e| e.kind == "callback").collect();
        assert_eq!(callbacks.len(), 2);
        assert_eq!(callbacks[0].parent.as_ref(), Some(&map.id));
        assert_eq!(callbacks[0].arguments[0].text, "2");
        assert_eq!(callbacks[0].result.as_ref().unwrap().text, "4");
        assert!(completed
            .iter()
            .any(|e| e.site.form == "(+ x x)" && e.site.line == 1));
        assert!(
            !completed.iter().any(|e| e.site.form.starts_with("(list")),
            "map's list construction is internal"
        );
    }

    #[tokio::test]
    async fn errors_unwind_only_the_affected_scopes_and_do_not_run_following_code() {
        let (mut f,events) = fiber("(def n 0) (try (dbg! (error \"bad\") (set n 99))) (dbg! (try (error \"caught\")) (+ n 1))");
        assert_eq!(crate::run(&mut f).await.unwrap(), Value::Int(1));
        let events = events.0.lock().unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|e| e.kind == "scope" && e.status == "error")
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e.kind == "scope" && e.status == "returned")
                .count(),
            1
        );
    }

    #[test]
    fn cancellation_keeps_partial_calls_and_does_not_continue() {
        let (mut f, events) = fiber("(def n 0) (dbg! (+ 1 2) (yield 5) (set n 99))");
        assert_eq!(f.start().unwrap(), Signal::Yield(Value::Int(5)));
        let env = f.global_env().clone();
        drop(f);
        assert_eq!(env.lock().unwrap().get(&"n".into()), Some(Value::Int(0)));
        let events = events.0.lock().unwrap();
        assert!(events
            .iter()
            .any(|e| e.site.form == "(+ 1 2)" && e.status == "returned"));
        assert!(events
            .iter()
            .any(|e| e.kind == "scope" && e.status == "cancelled"));
    }

    #[tokio::test]
    async fn quotation_and_macroexpansion_do_not_record_and_no_sink_preserves_scope() {
        let (mut f, events) = fiber("(macroexpand_1 '(dbg! (+ 1 2))) '(dbg! (+ 3 4))");
        crate::run(&mut f).await.unwrap();
        assert!(events.0.lock().unwrap().is_empty());
        let mut f =
            Fiber::<void::Void, ()>::from_expr("(begin (dbg! (def x 42)) x)", Env::standard(), ())
                .unwrap();
        assert_eq!(crate::run(&mut f).await.unwrap(), Value::Int(42));
    }

    #[tokio::test]
    async fn user_macro_generated_calls_keep_invocation_origin() {
        let (mut f, events) = fiber("(defmacro plus_one (x) `(+ ,x 1))\n(dbg! (plus_one! 4))");
        assert_eq!(crate::run(&mut f).await.unwrap(), Value::Int(5));
        let events = events.0.lock().unwrap();
        let generated = events.iter().find(|e| e.site.form == "(+ 4 1)").unwrap();
        assert_eq!(generated.site.file, "example.ll");
        assert_eq!(generated.site.line, 2);
        assert_eq!(generated.site.expression, "(plus_one! 4)");
        assert!(generated.site.generated);
    }

    #[tokio::test]
    async fn rejected_top_level_yield_is_an_error_in_the_recording() {
        let (mut f, events) = fiber("(dbg! (yield 1) (error \"must not run\"))");
        assert_eq!(
            crate::run(&mut f).await.unwrap_err(),
            crate::Error::UnexpectedTopLevelYield
        );
        let events = events.0.lock().unwrap();
        assert_eq!(events.iter().filter(|e| e.status == "error").count(), 1);
        assert!(events
            .last()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .text
            .contains("yield"));
    }

    #[test]
    fn large_values_are_bounded_without_dumping_closure_environments() {
        let value = Value::List(vec![Value::string(&"東京".repeat(10000)); 100]);
        let result = preview(&value);
        assert!(result.truncated);
        assert!(result.text.len() <= 1027);
    }
}
