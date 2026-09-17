import { test } from "node:test";
import assert from "node:assert/strict";
import { changedUniform } from "./uniform-cache.ts";
test("unchanged GPU inputs skip uploads but in-place array changes are detected", () => {
  const cache = new Map();
  assert.equal(changedUniform(cache, "fill", 0.5), true);
  assert.equal(changedUniform(cache, "fill", 0.5), false);
  assert.equal(changedUniform(cache, "fill", 0.51), true);
  const rectangle = [0, 0, 100, 50];
  assert.equal(changedUniform(cache, "rect", rectangle), true);
  assert.equal(changedUniform(cache, "rect", [...rectangle]), false);
  rectangle[2] = 101;
  assert.equal(changedUniform(cache, "rect", rectangle), true);
  assert.equal(changedUniform(cache, "rect", rectangle), false);
});
