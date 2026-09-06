import test from "node:test";
import assert from "node:assert/strict";
import { Navigation, rootPage, retentionMs } from "./navigation.mjs";
import { createOpening, createShowHandler, createTransport } from "./transport.mjs";

test("Tauri transport never overwrites reserved IPC envelope keys", async () => {
    const calls = [];
    const transport = createTransport(async (command, args = {}) => {
        for (const reserved of ["callback", "error", "cmd", "__tauriModule"]) {
            assert.equal(Object.hasOwn(args, reserved), false);
        }
        calls.push({command, args});
    });
    await transport.begin();
    await transport.query(rootPage(), 'quoted " input');
    await transport.dispatch('(:on_click (read_later_page))');
    await transport.close();
    assert.equal(calls[1].args.renderer, "root_items");
    assert.equal(calls[1].args.query, 'quoted " input');
    assert.equal(calls[3].command, "hide");
});

const tick = () => new Promise(resolve => setImmediate(resolve));

test("context hook completes before showing; cancelled openings stay hidden", async () => {
    const starts = [], shown = [];
    const nav = { visible: false, begin() {
        this.visible = true;
        return new Promise(resolve => starts.push(resolve));
    }};
    const open = createOpening(nav, () => shown.push("shown"));
    const first = open();
    assert.equal(shown.length, 0);
    nav.visible = false;
    const second = open();
    starts[0](); await first;
    assert.equal(shown.length, 0);
    starts[1](); await second;
    assert.equal(shown.length, 1);
    const cancelled = open(); nav.visible = false;
    starts[2](); await cancelled;
    assert.equal(shown.length, 1);
});
const item = title => ({ title, on_click: title });
function setup() {
    const queries = [], actions = [], starts = [], timers = new Map();
    let nextTimer = 0, closed = 0;
    const nav = new Navigation({
        begin: () => new Promise((resolve, reject) => starts.push({resolve, reject})),
        query: (page, text) => new Promise((resolve, reject) => queries.push({page, text, resolve, reject})),
        dispatch: form => new Promise((resolve, reject) => actions.push({form, resolve, reject})),
        close: () => closed++,
    }, () => {}, {
        setTimeout: fn => { timers.set(++nextTimer, fn); return nextTimer; },
        clearTimeout: id => timers.delete(id),
    });
    return {nav, queries, actions, starts, timers, closed: () => closed,
        fire: () => { const pending = [...timers.values()]; timers.clear(); pending.forEach(fn => fn()); }};
}

test("push is lazy and Back restores query, results, and selection", async () => {
    const t = setup(); t.nav.open(rootPage(), "Read");
    t.queries[0].resolve([item("Read Later"), item("Focus Window")]); await tick();
    t.nav.select(1);
    t.nav.push({...rootPage(), get_items: "read_later_items", prompt: "Read Later", debounce_ms: 200});
    assert.equal(t.queries.length, 2);
    t.nav.back();
    assert.equal(t.nav.current.query, "Read");
    assert.equal(t.nav.current.selected, 1);
    assert.equal(t.nav.current.items[0].title, "Read Later");
    t.queries[1].resolve([item("late")]); await tick();
    assert.equal(t.nav.current.items[0].title, "Read Later");
});

test("opening captures context once and preserves fast typing before it arrives", async () => {
    const t = setup(); t.nav.begin();
    t.nav.search("Focus"); t.nav.search("Focus Window");
    assert.equal(t.starts.length, 1);
    assert.equal(t.queries.length, 0);
    const page = {...rootPage(), args: "(((:os/window :id 7)))"};
    t.starts[0].resolve({type: "push_page", page}); await tick();
    assert.equal(t.queries.length, 1);
    assert.equal(t.queries[0].text, "Focus Window");
    assert.equal(t.queries[0].page.args, page.args);
    t.queries[0].resolve([item("Focus Window")]); await tick();
    assert.equal(t.nav.snapshot().loading, false);
});

test("Escape during context capture never opens a late page; a new session recovers", async () => {
    const t = setup(); t.nav.begin(); t.nav.back();
    t.starts[0].resolve({type: "push_page", page: rootPage()}); await tick();
    assert.equal(t.queries.length, 0);
    assert.equal(t.nav.visible, false);
    t.nav.begin(); t.starts[1].reject(new Error("service unavailable")); await tick();
    assert.match(t.nav.snapshot().error, /service unavailable/);
    t.nav.search("retry");
    assert.equal(t.queries.length, 1);
});

test("multi-argument pages keep whole opaque arguments and Back restores the prefix", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("Move Window")]); await tick();
    t.nav.activate();
    const first = {...rootPage(), get_items:"call_items", args:"(move_window ())"};
    t.actions[0].resolve({type:"push_page", page:first}); await tick();
    t.queries[1].resolve([item("Safari")]); await tick(); t.nav.activate();
    const second = {...first, args:"(move_window ((:os/window :id 7)))"};
    t.actions[1].resolve({type:"push_page", page:second}); await tick();
    assert.equal(t.queries[2].page.args, second.args);
    t.nav.back();
    assert.equal(t.nav.current.page.args, first.args);
    assert.equal(t.nav.current.items[0].title, "Safari");
    t.nav.back();
    assert.equal(t.nav.current.page.get_items, "root_items");
});

test("debounce coalesces input and never accumulates in-flight renders", async () => {
    const t = setup(); t.nav.open({...rootPage(), debounce_ms: 200});
    for (const q of ["e", "em", "emacs"]) t.nav.search(q);
    assert.equal(t.timers.size, 1); t.fire();
    assert.equal(t.queries.length, 1);
    t.queries[0].resolve([item("obsolete")]); await tick();
    assert.equal(t.queries.length, 2);
    assert.equal(t.queries[1].text, "emacs");
    assert.equal(t.nav.current.items.length, 0);
    t.queries[1].resolve([item("correct")]); await tick();
    assert.equal(t.nav.snapshot().loading, false);
    assert.equal(t.nav.current.items[0].title, "correct");
});

test("Escape drops pending timers and late action responses", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("Read Later")]); await tick();
    t.nav.activate(); t.nav.activate();
    assert.equal(t.actions.length, 1);
    t.nav.back(); assert.equal(t.closed(), 1);
    t.actions[0].resolve({type:"push_page", page:rootPage()}); await tick();
    assert.equal(t.nav.visible, false);
    t.nav.open({...rootPage(), debounce_ms:200});
    t.nav.search("pending"); t.nav.close();
    assert.equal(t.timers.size, 0);
});

test("errors leave the GUI usable and a later query can recover", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].reject(new Error("disconnected")); await tick();
    assert.match(t.nav.snapshot().error, /disconnected/);
    t.nav.search("retry");
    t.queries[1].resolve([item("works")]); await tick();
    t.nav.activate(); t.actions[0].reject(new Error("action failed")); await tick();
    assert.equal(t.nav.visible, true);
    assert.match(t.nav.snapshot().error, /action failed/);
    t.nav.activate(); t.actions[1].resolve({type:"close"}); await tick();
    assert.equal(t.closed(), 1);
});

test("a secondary action dispatches its own opaque command on the selected row", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([{...item("Article"), actions: [item("Copy URL")]}]); await tick();
    t.nav.activate(0, 0);
    assert.equal(t.actions[0].form, "Copy URL");
    t.actions[0].resolve({type: "close"}); await tick();
    assert.equal(t.closed(), 1);
});

test("a failed query cannot re-enable stale actions from the previous query", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("old action")]); await tick();
    t.nav.search("different");
    t.queries[1].reject(new Error("unavailable")); await tick();
    assert.equal(t.nav.current.items.length, 0);
    t.nav.activate();
    assert.equal(t.actions.length, 0);
});

test("stale same-text response after reopening cannot overwrite current frame", async () => {
    const t = setup(); t.nav.open(); t.nav.close(); t.nav.open();
    t.queries[0].resolve([item("old session")]); await tick();
    assert.equal(t.queries.length, 2);
    assert.equal(t.nav.current.items.length, 0);
    t.queries[1].resolve([item("new session")]); await tick();
    assert.equal(t.nav.current.items[0].title, "new session");
});

test("reopen retains the page for eight minutes while refreshing root context", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("Read Later")]); await tick();
    t.nav.push({...rootPage(), get_items: "read_later_items"}, "emacs");
    t.queries[1].resolve([item("first"), item("second")]); await tick();
    t.nav.select(1); t.nav.close(); t.nav.begin();
    assert.equal(t.nav.current.query, "emacs");
    assert.equal(t.nav.current.selected, 1);
    t.starts[0].resolve({type: "push_page", page: {...rootPage(), args: "(((:os/window :id 8)))"}}); await tick();
    assert.equal(t.queries.length, 2, "cached subpage isn't reloaded on reopen");
    assert.equal(t.nav.frames[0].page.args, "(((:os/window :id 8)))");
    t.nav.close(); t.nav.hiddenAt -= retentionMs; t.nav.begin();
    assert.equal(t.nav.frames.length, 1);
    assert.equal(t.nav.current.query, "");
});

test("reopening a root page never dispatches stale context while refreshing", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("Save old page")]); await tick();
    t.nav.close(); t.nav.begin(); t.nav.search("Save"); t.nav.activate();
    assert.equal(t.actions.length, 0);
    assert.equal(t.queries.length, 1);
    t.starts[0].resolve({type: "push_page", page: {...rootPage(), args: "(((:web/page :url new)))"}}); await tick();
    assert.equal(t.queries[1].text, "Save");
    assert.equal(t.queries[1].page.args, "(((:web/page :url new)))");
});

const inputPage = id => ({...rootPage(), get_items: "function_items", args: `("${id}")`,
    on_cancel: `(:on_click (cancel_input "${id}"))`});

test("show bursts share an opening and never toggle a visible input closed", async () => {
    const t = setup(), shown = [];
    const show = createShowHandler(t.nav, () => shown.push(true));
    const first = show();
    assert.equal(show(), first);
    assert.equal(t.starts.length, 1);
    t.starts[0].resolve({type:"push_page", page:inputPage("one")}); await first;
    t.queries[0].resolve([item("function")]); await tick();
    t.nav.push({...rootPage(), get_items:"fill_call_items"}, "typed argument");
    await show();
    assert.equal(t.starts.length, 1);
    assert.equal(t.nav.current.query, "typed argument");
    assert.equal(t.nav.visible, true);
    assert.equal(t.closed(), 0);
    assert.equal(shown.length, 2);
});

test("pending input overrides retained ordinary history, and the same input resumes", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([]); await tick();
    t.nav.push({...rootPage(), get_items:"ordinary_items"});
    t.queries[1].resolve([]); await tick();
    t.nav.suspend(); t.nav.begin();
    t.starts[0].resolve({type:"push_page", page:inputPage("one")}); await tick();
    assert.equal(t.nav.frames.length, 1);
    assert.equal(t.nav.current.page.get_items, "function_items");
    t.queries[2].resolve([]); await tick();
    t.nav.push({...rootPage(), get_items:"fill_call_items"}, "argument");
    t.queries[3].resolve([]); await tick();
    t.nav.suspend(); t.nav.begin();
    t.starts[1].resolve({type:"push_page", page:inputPage("one")}); await tick();
    assert.equal(t.nav.current.query, "argument");
    assert.equal(t.nav.frames.length, 2);
    // Cancellation elsewhere must not resurrect an abandoned input page.
    t.nav.suspend(); t.nav.begin();
    t.starts[2].resolve({type:"push_page", page:rootPage()}); await tick();
    assert.equal(t.nav.frames.length, 1);
    assert.equal(t.nav.current.page.get_items, "root_items");
});

test("Escape cancels an input only when leaving its page stack; completion doesn't cancel", async () => {
    const t = setup(); t.nav.open(inputPage("one"));
    t.queries[0].resolve([]); await tick();
    t.nav.push({...rootPage(), get_items:"fill_call_items"});
    t.queries[1].resolve([]); await tick();
    t.nav.back();
    assert.equal(t.actions.length, 0);
    t.nav.back();
    assert.equal(t.actions[0].form, inputPage("one").on_cancel);
    assert.equal(t.closed(), 1);
    t.nav.open(inputPage("two"));
    t.queries[2].resolve([item("finish")]); await tick();
    t.nav.activate();
    t.actions[1].resolve({type:"close"}); await tick();
    assert.equal(t.actions.length, 2, "successful completion doesn't also dispatch cancel");
});

test("reconnect checks pending work without showing an idle palette", async () => {
    const t = setup(), shown = [];
    const show = createShowHandler(t.nav, () => shown.push(true));
    const idle = show({background:true});
    t.starts[0].resolve({type:"push_page", page:rootPage()}); await idle;
    assert.equal(t.nav.visible, false);
    assert.equal(shown.length, 0);
    const pending = show({background:true});
    t.starts[1].resolve({type:"push_page", page:inputPage("waiting")}); await pending;
    assert.equal(t.nav.visible, true);
    assert.equal(shown.length, 1);
    assert.equal(t.queries[0].page.get_items, "function_items");
});
