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
const lava = (setup) => setup.assignments.idle.patterns[0].settings;

test("equal Lava rates keep light and flow synchronized through inputs", () => {
  for (const rate of [undefined, 0.4, 1, 2]) {
    const setup = assignedSetup();
    lava(setup).flowSpeed = rate ?? 1;
    lava(setup).lightRotation = rate ?? 1;
    const player = new ExpressionPlayer(setup);
    for (const action of ["idle", "typing", "working", "idle", "complete"]) {
      player.trigger(action);
      const frame = advance(player, 0.75);
      assert.deepEqual(frame.u_lavaPhase, [frame.u_phase, frame.u_phase]);
    }
  }
});

test("light and flow can stop independently and rate edits preserve the current pose", () => {
  const setup = assignedSetup();
  Object.assign(lava(setup), { lightRotation: 0, flowSpeed: 0.4 });
  const player = new ExpressionPlayer(setup);
  const before = advance(player, 2).u_lavaPhase;
  assert.equal(before[0], 0);
  assert.ok(before[1] > 0);
  Object.assign(lava(setup), { lightRotation: 1, flowSpeed: 0 });
  assert.deepEqual(player.frame().u_lavaPhase, before);
  const after = advance(player, 2).u_lavaPhase;
  assert.ok(after[0] > 0);
  assert.equal(after[1], before[1]);
});

test("Nudge briefly advances both components even when their idle rates are zero", () => {
  const setup = assignedSetup();
  Object.assign(lava(setup), { lightRotation: 0, flowSpeed: 0 });
  const player = new ExpressionPlayer(setup);
  assert.deepEqual(advance(player, 1).u_lavaPhase, [0, 0]);
  player.trigger("typing");
  const kicked = advance(player, 0.5).u_lavaPhase;
  assert.ok(kicked[0] > 0);
  assert.equal(kicked[0], kicked[1]);
  assert.deepEqual(advance(player, 1).u_lavaPhase, kicked);
});
