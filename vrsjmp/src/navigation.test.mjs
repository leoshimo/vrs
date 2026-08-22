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

test("initial page resolves before showing; cancelled openings stay hidden", async () => {
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
const item = title => ({ id: title, title, on_click: title });
function setup() {
    const queries = [], actions = [], starts = [], timers = new Map();
    let nextTimer = 0, closed = 0, now = 0;
    const nav = new Navigation({
        begin: () => new Promise((resolve, reject) => starts.push({resolve, reject})),
        query: (page, text) => new Promise((resolve, reject) => queries.push({page, text, resolve, reject})),
        dispatch: form => new Promise((resolve, reject) => actions.push({form, resolve, reject})),
        close: () => closed++,
    }, () => {}, {
        now: () => now,
        setTimeout: (fn, delay) => { timers.set(++nextTimer, {fn, at: now + delay}); return nextTimer; },
        clearTimeout: id => timers.delete(id),
    });
    const advance = ms => {
        const end = now + ms;
        while (timers.size) {
            const [id, timer] = [...timers].sort((a, b) => a[1].at - b[1].at)[0];
            if (timer.at > end) break;
            now = timer.at;
            timers.delete(id);
            timer.fn();
        }
        now = end;
    };
    return {nav, queries, actions, starts, timers, advance, closed: () => closed};
}

test("activating a row selects it and Back restores query, results, and selection", async () => {
    const t = setup(); t.nav.open(rootPage(), "Read");
    t.queries[0].resolve([item("Read Later"), item("Focus Window")]); await tick();
    let rendered;
    t.nav.render = state => { rendered = state; };
    t.nav.activate(1);
    assert.equal(t.actions[0].form, "Focus Window");
    assert.equal(rendered.selected, 1, "the activated row is highlighted while dispatch is pending");
    t.actions[0].resolve({type: "push_page", page: {...rootPage(), get_items: "read_later_items", prompt: "Read Later", debounce_ms: 200}});
    await tick();
    assert.equal(t.queries.length, 2);
    t.nav.back();
    assert.equal(t.nav.current.query, "Read");
    assert.equal(t.nav.current.selected, 1);
    assert.equal(t.nav.current.items[0].title, "Read Later");
    t.queries[1].resolve([item("late")]); await tick();
    assert.equal(t.nav.current.items[0].title, "Read Later");
});

test("opening requests its page once and preserves fast typing before it arrives", async () => {
    const t = setup(); t.nav.begin();
    t.nav.search("Focus"); t.nav.search("Focus Window");
    assert.equal(t.starts.length, 1);
    assert.equal(t.queries.length, 0);
    const page = {...rootPage(), get_items: "request_items", args: '("request-7")'};
    t.starts[0].resolve({type: "push_page", page}); await tick();
    assert.equal(t.queries.length, 1);
    assert.equal(t.queries[0].text, "Focus Window");
    assert.equal(t.queries[0].page.args, page.args);
    t.queries[0].resolve([item("Focus Window")]); await tick();
    assert.equal(t.nav.snapshot().loading, false);
});

test("Escape while choosing the initial page never opens a late page; a new session recovers", async () => {
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
    assert.equal(t.timers.size, 2); t.advance(200);
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
    assert.equal(t.timers.size, 1, "only the idle reset remains after dismissal");
});

test("errors leave the GUI usable and a later query can recover", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].reject(new Error("disconnected")); await tick();
    assert.match(t.nav.snapshot().error, /disconnected/);
    t.nav.search("retry");
    t.queries[1].resolve([item("works")]); await tick();
    t.nav.activate(); t.actions[0].reject(new Error("action failed")); await tick();
    assert.equal(t.nav.visible, true);
    assert.equal(t.nav.current.query, "retry");
    assert.match(t.nav.snapshot().error, /action failed/);
    t.nav.activate(); t.actions[1].resolve({type:"close"}); await tick();
    assert.equal(t.closed(), 1);
});

test("a secondary action dispatches its own opaque command on the selected row", async () => {
    const t = setup(); t.nav.open(rootPage(), "Article");
    t.queries[0].resolve([{...item("Article"), actions: [item("Copy URL")]}]); await tick();
    t.nav.activate(0, 0);
    assert.equal(t.actions[0].form, "Copy URL");
    t.actions[0].resolve({type: "close"}); await tick();
    assert.equal(t.closed(), 1);
    assert.equal(t.nav.current.query, "");
});

test("refresh keeps the page, query, and selected command as current labels move", async () => {
    const t = setup(); t.nav.open(rootPage(), "Display");
    t.queries[0].resolve([item("Display Resolution")]); await tick();
    const page = {...rootPage(), get_items: "display_items"};
    t.nav.push(page, "60Hz");
    const rows = [item("1470x956"), item("1710x1112")];
    t.queries[1].resolve(rows); await tick();
    t.nav.activate(1);
    t.actions[0].resolve({type: "refresh"}); await tick();
    assert.equal(t.nav.visible, true);
    assert.equal(t.closed(), 0);
    assert.equal(t.nav.frames.length, 2);
    assert.equal(t.queries[2].text, "60Hz");
    assert.equal(t.queries[2].page, page);
    // Preserve identity even if fresh labels reorder the search results.
    t.queries[2].resolve([{...rows[1], title: "1710x1112 (current)", on_click: "updated row"}, rows[0]]); await tick();
    assert.equal(t.nav.current.query, "60Hz");
    assert.equal(t.nav.current.selected, 0);
    assert.equal(t.nav.current.items[0].title, "1710x1112 (current)");
    t.nav.activate(); t.actions[1].resolve({type: "refresh"}); await tick();
    t.queries[3].resolve([rows[0], {...rows[1], title: "1710x1112 (current)"}]); await tick();
    assert.equal(t.nav.current.selected, 1);
    t.nav.back();
    assert.equal(t.nav.current.query, "Display");
});

test("refresh arriving while hidden preserves the query and reloads on reopen", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("Display Resolution")]); await tick();
    t.nav.push({...rootPage(), get_items: "display_items"}, "60Hz");
    const rows = [item("first"), item("second")];
    t.queries[1].resolve(rows); await tick();
    t.nav.activate(1); t.nav.suspend();
    t.actions[0].resolve({type: "refresh"}); await tick();
    assert.equal(t.nav.visible, false);
    assert.equal(t.nav.current.query, "60Hz");
    assert.equal(t.nav.current.loaded, false);
    assert.equal(t.queries.length, 2);
    t.nav.begin(); t.starts[0].resolve({type: "push_page", page: rootPage()}); await tick();
    assert.equal(t.queries[2].text, "60Hz");
    t.queries[2].resolve(rows); await tick();
    assert.equal(t.nav.current.selected, 1);
});

for (const subpage of [false, true]) {
    for (const blur of [false, true]) {
        test(`running clears the query and refreshes on reopen (subpage=${subpage}, blur=${blur})`, async () => {
            const t = setup(); t.nav.open(rootPage(), "Browse");
            t.queries[0].resolve([item("Browse Functions")]); await tick();
            if (subpage) {
                t.nav.push({...rootPage(), get_items: "service_function_items"}, "get_windows");
                t.queries[1].resolve([item("get_windows")]); await tick();
            }
            const page = t.nav.current.page;
            t.nav.activate();
            if (blur) t.nav.suspend();
            t.actions[0].resolve({type: "close"}); await tick();
            assert.equal(t.nav.visible, false);
            assert.equal(t.nav.current.query, "");
            assert.equal(t.nav.current.items.length, 0);
            t.nav.begin();
            t.starts[0].resolve({type: "push_page", page: rootPage()}); await tick();
            assert.equal(t.queries.at(-1).text, "");
            assert.equal(t.queries.at(-1).page.get_items, page.get_items);
            t.queries.at(-1).resolve([item("fresh")]); await tick();
            assert.equal(t.nav.current.items[0].title, "fresh");
        });
    }
}

test("an old action finishing after reopening cannot clear a new query", async () => {
    const t = setup(); t.nav.open(rootPage(), "old");
    t.queries[0].resolve([item("old action")]); await tick();
    t.nav.activate(); t.nav.suspend(); t.nav.begin();
    t.starts[0].resolve({type: "push_page", page: rootPage()}); await tick();
    t.nav.search("new");
    t.actions[0].resolve({type: "close"}); await tick();
    assert.equal(t.nav.visible, true);
    assert.equal(t.nav.current.query, "new");
    assert.equal(t.closed(), 0);
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

test("reopen retains the page for 30 seconds while refreshing the initial page", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("Read Later")]); await tick();
    t.nav.push({...rootPage(), get_items: "read_later_items"}, "emacs");
    t.queries[1].resolve([item("first"), item("second")]); await tick();
    t.nav.select(1); t.nav.close(); t.advance(29_999); t.nav.begin();
    assert.equal(t.nav.current.query, "emacs");
    assert.equal(t.nav.current.selected, 1);
    t.starts[0].resolve({type: "push_page", page: {...rootPage(), args: '("request-8")'}}); await tick();
    assert.equal(t.queries.length, 2, "cached subpage isn't reloaded on reopen");
    assert.equal(t.nav.frames[0].page.args, '("request-8")');
    t.nav.close(); t.advance(30_000);
    assert.equal(t.nav.frames.length, 0, "hidden navigation expires without reopening");
    assert.equal(t.nav.visible, false);
    assert.equal(t.starts.length, 1, "expiring hidden state does not contact the service");
    t.nav.begin();
    assert.equal(t.nav.frames.length, 1);
    assert.equal(t.nav.current.query, "");
});

test("visible idle navigation resets 30 seconds after the last interaction", async () => {
    assert.equal(retentionMs, 30_000);
    const t = setup(); t.nav.open();
    t.queries[0].resolve([]); await tick();
    t.nav.push({...rootPage(), get_items: "note_items"}, "notes");
    t.queries[1].resolve([item("first"), item("second")]); await tick();
    t.advance(29_000); t.nav.select(1);
    t.advance(29_000);
    assert.equal(t.nav.current.page.get_items, "note_items");
    t.nav.search("new search");
    t.queries[2].resolve([item("match")]); await tick();
    t.advance(29_000); t.nav.interact(); // Scrolling or action-menu interaction.
    t.advance(29_999);
    assert.equal(t.nav.current.query, "new search");
    t.advance(1);
    assert.equal(t.nav.visible, true);
    assert.equal(t.nav.frames.length, 1);
    assert.equal(t.nav.current.query, "");
    assert.equal(t.nav.current.selected, 0);
    t.starts[0].resolve({type: "push_page", page: rootPage()}); await tick();
    t.queries[3].resolve([item("Home command")]); await tick();
    t.advance(60_000);
    assert.equal(t.starts.length, 1, "the idle reset does not keep refreshing Home");
    assert.equal(t.nav.current.page.get_items, "root_items");
});

test("dismissal starts a fresh 30 seconds even after prolonged visible use", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([]); await tick();
    t.nav.push({...rootPage(), get_items: "note_items"});
    t.queries[1].resolve([]); await tick();
    t.advance(29_000); t.nav.suspend();
    t.advance(29_999);
    assert.equal(t.nav.frames.length, 2);
    t.advance(1);
    assert.equal(t.nav.frames.length, 0);
});

test("idle reset waits for an action and drops a late query response", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("Browse Notes")]); await tick();
    t.nav.activate(); t.advance(30_000);
    assert.equal(t.nav.snapshot().loading, true);
    assert.equal(t.starts.length, 0);
    t.actions[0].resolve({type: "push_page", page: {...rootPage(), get_items: "note_items"}});
    await tick(); t.advance(0);
    assert.equal(t.nav.current.page.get_items, "root_items");
    t.starts[0].resolve({type: "push_page", page: rootPage()}); await tick();
    t.queries[1].resolve([item("late note")]); await tick();
    assert.equal(t.nav.current.items.length, 0);
    t.queries[2].resolve([item("Home")]); await tick();
    assert.equal(t.nav.current.items[0].title, "Home");
});

test("reopening a root page never dispatches stale items while refreshing", async () => {
    const t = setup(); t.nav.open();
    t.queries[0].resolve([item("Old request")]); await tick();
    t.nav.close(); t.nav.begin(); t.nav.search("Save"); t.nav.activate();
    assert.equal(t.actions.length, 0);
    assert.equal(t.queries.length, 1);
    t.starts[0].resolve({type: "push_page", page: {...rootPage(), args: '("request-new")'}}); await tick();
    assert.equal(t.queries[1].text, "Save");
    assert.equal(t.queries[1].page.args, '("request-new")');
});

const inputPage = id => ({...rootPage(), get_items: "function_items", args: `("${id}")`,
    on_cancel: `(:on_click (cancel_input "${id}"))`});

test("idle reset discards input navigation without cancelling its pending request", async () => {
    const t = setup(); t.nav.open(inputPage("waiting"));
    t.queries[0].resolve([]); await tick();
    t.nav.push({...rootPage(), get_items: "fill_call_items"}, "old argument");
    t.queries[1].resolve([]); await tick();
    t.advance(30_000);
    assert.equal(t.actions.length, 0, "idleness does not dispatch cancellation");
    t.starts[0].resolve({type: "push_page", page: inputPage("waiting")}); await tick();
    assert.equal(t.nav.frames.length, 1);
    assert.equal(t.nav.current.query, "");
    assert.equal(t.nav.current.page.on_cancel, inputPage("waiting").on_cancel);
    t.queries[2].resolve([]); await tick();
    t.nav.suspend(); t.advance(30_000);
    assert.equal(t.nav.frames.length, 0);
    assert.equal(t.actions.length, 0);
    t.nav.begin();
    t.starts[1].resolve({type: "push_page", page: inputPage("waiting")}); await tick();
    assert.equal(t.nav.current.page.on_cancel, inputPage("waiting").on_cancel);
});

test("a dismissed opening that finishes after the deadline still expires", async () => {
    const t = setup(); t.nav.begin(); t.nav.suspend();
    t.advance(30_000);
    t.starts[0].resolve({type: "push_page", page: rootPage()}); await tick();
    t.advance(0);
    assert.equal(t.nav.frames.length, 0);
    assert.equal(t.nav.visible, false);
    assert.equal(t.timers.size, 0);
});

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
