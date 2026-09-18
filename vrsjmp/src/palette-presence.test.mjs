import { test } from "node:test";
import assert from "node:assert/strict";
import { PalettePresence, presentedSnapshot } from "./palette-presence.mjs";

const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const tick = () => new Promise(resolve => setImmediate(resolve));
function fixture(native = {}) {
  const calls = [], animations = [], hidden = [];
  const presence = new PalettePresence({
    show: () => { calls.push("show"); return native.show?.(); },
    hide: () => { calls.push("hide"); return native.hide?.(); },
    prepare: () => calls.push("prepare"),
    onHidden: () => hidden.push(true),
    animate: (appearing, reason) => {
      const done = deferred();
      const animation = { appearing, reason, finished: done.promise, finish: done.resolve, cancel: done.resolve };
      animations.push(animation);
      return animation;
    },
  });
  return { presence, calls, animations, hidden };
}

test("completion keeps the renderer alive until native hide and can be interrupted", async () => {
  const t = fixture();
  await t.presence.show();
  const closing = t.presence.hide("complete");
  assert.equal(t.animations.at(-1).reason, "complete");
  assert.equal(t.hidden.length, 0);
  await t.presence.show();
  await closing;
  assert.equal(t.hidden.length, 0);
  const completed = t.presence.hide("complete");
  t.animations.at(-1).finish();
  await completed;
  assert.equal(t.hidden.length, 1);
  assert.equal(t.calls.at(-1), "hide");
});

test("hide keeps the native window until the exit completes", async () => {
  const t = fixture();
  await t.presence.show();
  const closing = t.presence.hide();
  await tick();
  assert.deepEqual(t.calls, ["prepare", "show"]);
  assert.equal(t.animations.at(-1).appearing, false);
  t.animations.at(-1).finish();
  await closing;
  assert.equal(t.calls.at(-1), "hide");
});

test("reopening during exit cancels its pending native hide", async () => {
  const t = fixture();
  await t.presence.show();
  const closing = t.presence.hide();
  await t.presence.show();
  await closing;
  assert.equal(t.calls.includes("hide"), false);
  assert.equal(t.animations.at(-1).appearing, true);
  assert.equal(t.presence.visible, true);
});

test("hide while native show is pending finishes hidden", async () => {
  const nativeShow = deferred();
  const t = fixture({ show: () => nativeShow.promise });
  const opening = t.presence.show();
  await tick();
  const closing = t.presence.hide();
  t.animations.at(-1).finish();
  await tick();
  assert.equal(t.calls.includes("hide"), false);
  nativeShow.resolve();
  await Promise.all([opening, closing]);
  assert.equal(t.calls.at(-1), "hide");
  assert.equal(t.animations.some(a => a.appearing), false);
});

test("reopen waits for an already dispatched native hide", async () => {
  const nativeHide = deferred();
  const t = fixture({ hide: () => nativeHide.promise });
  await t.presence.show();
  const closing = t.presence.hide();
  t.animations.at(-1).finish();
  await tick();
  const opening = t.presence.show();
  await tick();
  assert.equal(t.calls.filter(c => c === "show").length, 1);
  nativeHide.resolve();
  await Promise.all([opening, closing]);
  assert.equal(t.calls.at(-1), "show");
  assert.equal(t.animations.at(-1).appearing, true);
});

test("repeated show during opening does not lose the entrance animation", async () => {
  const nativeShow = deferred();
  const t = fixture({ show: () => nativeShow.promise });
  const first = t.presence.show(), second = t.presence.show();
  nativeShow.resolve();
  await Promise.all([first, second]);
  assert.equal(t.animations.length, 1);
  assert.equal(t.animations[0].appearing, true);
});

test("disposing cancels a delayed hide", async () => {
  const t = fixture();
  await t.presence.show();
  const closing = t.presence.hide();
  t.presence.dispose();
  await closing;
  assert.equal(t.calls.includes("hide"), false);
});

test("a failed native operation does not block subsequent visibility changes", async () => {
  let failed = false;
  const t = fixture({ hide: () => {
    if (!failed) { failed = true; throw new Error("native error"); }
  } });
  await t.presence.show();
  const closing = t.presence.hide();
  t.animations.at(-1).finish();
  await assert.rejects(closing, /native error/);
  await t.presence.show();
  assert.equal(t.animations.at(-1).appearing, true);
});

const previous = { visible: true, page: { title: "Home" }, query: "", items: [{ title: "Read Later" }], loading: false };
test("new page loading retains the old rows until results arrive", () => {
  const pending = { visible: true, page: { title: "Read Later" }, query: "recent", items: [], loading: true };
  const shown = presentedSnapshot(pending, previous);
  assert.equal(shown.items, previous.items);
  assert.equal(shown.page, previous.page);
  assert.equal(shown.query, "recent");
  assert.equal(shown.loading, true);
  const ready = { ...pending, loading: false, items: [{ title: "Article" }] };
  assert.equal(presentedSnapshot(ready, shown), ready);
});

test("empty and failed results replace retained rows", () => {
  for (const error of ["", "Offline"]) {
    const ready = { ...previous, items: [], error };
    assert.equal(presentedSnapshot(ready, previous), ready);
  }
});

test("dismiss retains the complete view after an action clears navigation", () => {
  const working = { ...previous, query: "save", loading: true };
  const hidden = { ...previous, visible: false, items: [], query: "", loading: false };
  assert.deepEqual(presentedSnapshot(hidden, working), { ...working, visible: false });
});

test("a new opening does not borrow stale rows from a hidden page", () => {
  const opening = { ...previous, items: [], loading: true };
  assert.equal(presentedSnapshot(opening, { ...previous, visible: false }), opening);
});
