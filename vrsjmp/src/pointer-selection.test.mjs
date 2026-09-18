import { test } from "node:test";
import assert from "node:assert/strict";
import { pointerSelection } from "./pointer-selection.mjs";

test("scrolling or replacing rows under a stationary pointer does not select them", () => {
  const moved = pointerSelection();
  const cursor = { screenX: 80, screenY: 140, movementX: 0, movementY: 0 };
  assert.equal(moved(cursor), false);
  assert.equal(moved({ ...cursor, screenY: 150 }), true);
  assert.equal(moved({ ...cursor, screenY: 150 }), false);
  assert.equal(moved({ ...cursor, screenX: 81, screenY: 150 }), true);
});

test("showing the window under the cursor does not select the hovered row", () => {
  const moved = pointerSelection();
  const cursor = { screenX: 700, screenY: 500, movementX: 300, movementY: 200 };
  assert.equal(moved(cursor), false);
  assert.equal(moved(cursor), false);
  assert.equal(moved({ ...cursor, screenX: 701 }), true);
});

test("moving the window does not count as moving the cursor", () => {
  const moved = pointerSelection();
  const cursor = { screenX: 700, screenY: 500, clientX: 100, clientY: 200 };
  moved(cursor);
  assert.equal(moved({ ...cursor, clientX: 150, clientY: 250, movementX: 50, movementY: 50 }), false);
  assert.equal(moved({ ...cursor, screenY: 501 }), true);
});

test("reopening ignores cursor movement made while the palette was hidden", () => {
  const moved = pointerSelection();
  moved({ screenX: 700, screenY: 500 });
  moved({ screenX: 710, screenY: 500 });
  moved.reset();
  assert.equal(moved({ screenX: 800, screenY: 600, movementX: 90, movementY: 100 }), false);
  assert.equal(moved({ screenX: 800, screenY: 600 }), false);
  assert.equal(moved({ screenX: 800, screenY: 601 }), true);
});
