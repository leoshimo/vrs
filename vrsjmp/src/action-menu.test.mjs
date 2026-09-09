import test from "node:test";
import assert from "node:assert/strict";
import { menuEntries, defaultMenuSelection } from "./action-menu.mjs";

const item = { actions: [
    { title: "Copy URL", primary: false },
    { title: "Open in Browser", primary: true },
    { title: "Copy Title and URL", primary: false },
] };

test("opening selects the first secondary action and preserves dispatch indices", () => {
    const entries = menuEntries(item);
    assert.deepEqual(entries.map(entry => entry.index), [1, 0, 2]);
    assert.equal(defaultMenuSelection(entries), 1);
    assert.equal(entries[defaultMenuSelection(entries)].index, 0);
    const onlySecondary = menuEntries({actions: [{title: "Fill arguments"}]});
    assert.equal(defaultMenuSelection(onlySecondary), 0);
    const onlyPrimary = menuEntries({actions: [{title: "Open", primary: true}]});
    assert.equal(defaultMenuSelection(onlyPrimary), 0);
    assert.equal(defaultMenuSelection([]), 0);
});

test("filtering can select the primary and still dispatches the original action", () => {
    const copies = menuEntries(item, "cpy url");
    assert.deepEqual(copies.map(entry => entry.index), [0, 2]);
    assert.equal(defaultMenuSelection(copies, "cpy url"), 0);
    const primary = menuEntries(item, "brwsr");
    assert.deepEqual(primary.map(entry => entry.index), [1]);
    assert.equal(defaultMenuSelection(primary, "brwsr"), 0);
    assert.deepEqual(menuEntries(item, "missing"), []);
    assert.deepEqual(item.actions.map(action => action.title), ["Copy URL", "Open in Browser", "Copy Title and URL"]);
});
