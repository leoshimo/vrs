import { test } from "node:test";
import assert from "node:assert/strict";
import { register } from "node:module";
register("./test-resolve.mjs", import.meta.url);
const { ExpressionPlayer } = await import("./expression-player.ts");
const { assignedSetup } = await import("./assigned-setup.ts");
const advance = (player, seconds) => {
  for (let i = 0; i < Math.round(seconds * 120); i++) player.advance(1 / 120);
  return player.frame();
};
const withDrift = () => {
  const setup = assignedSetup();
  const working = setup.assignments.working;
  working.patterns = [{ kind: "drift", settings: { speed: 0.7, strength: 0.8 } }];
  return setup;
};

test("Lava and the assigned reactions never translate the print grid through inactive Drift", () => {
  const player = new ExpressionPlayer(assignedSetup());
  for (const action of ["idle", "typing", "working", "idle", "complete"]) {
    player.trigger(action);
    const frame = advance(player, 2);
    assert.equal(frame.u_printOffset[0], 0);
  }
});

test("Drift starts from rest even after a long idle and rate edits do not reposition the grid", () => {
  const early = new ExpressionPlayer(withDrift());
  const late = new ExpressionPlayer(withDrift());
  advance(late, 30);
  assert.equal(late.frame().u_printOffset[0], 0);
  early.trigger("working");
  late.trigger("working");
  const a = advance(early, 0.5).u_printOffset[0];
  const b = advance(late, 0.5).u_printOffset[0];
  assert.ok(a > 0);
  assert.equal(a, b);
  late.setup.assignments.working.patterns[0].settings.speed = 1.8;
  assert.equal(late.frame().u_printOffset[0], b);
});

test("Drift coasts to a stationary grid when Working ends", () => {
  const player = new ExpressionPlayer(withDrift());
  player.trigger("working");
  const moving = advance(player, 2).u_printOffset[0];
  player.trigger("idle");
  const settled = advance(player, 4).u_printOffset[0];
  assert.ok(settled > moving);
  assert.ok(Math.abs(advance(player, 4).u_printOffset[0] - settled) < 1e-12);
});
