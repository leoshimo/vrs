import { test } from "node:test";
import assert from "node:assert/strict";
import { register } from "node:module";
register("./test-resolve.mjs", import.meta.url);
const { ExpressionPlayer } = await import("./expression-player.ts");
const { assignedSetup } = await import("./assigned-setup.ts");
const { readPattern } = await import("./effect-settings.ts");
const step = (p, seconds) => {
  for (let i = 0; i < Math.round(seconds * 120); i++) p.advance(1 / 120);
  return p.frame();
};

test("sampling a frame cannot change animation state or consume responses", () => {
  const p = new ExpressionPlayer(assignedSetup());
  p.trigger("working");
  p.trigger("submit");
  step(p, 0.15);
  const before = p.frame(),
    time = p.time,
    signals = p.signals();
  for (let i = 0; i < 10; i++) assert.deepEqual(p.frame(), before);
  assert.equal(p.time, time);
  assert.deepEqual(p.signals(), signals);
});
test("two Lava contributions retain their individual rates in either order", () => {
  const setup = assignedSetup(),
    e = setup.assignments.idle;
  e.patterns = [
    { kind: "lava", settings: { flowSpeed: 0.3, lightRotation: 2, flowStrength: 0.4 } },
    { kind: "lava", settings: { flowSpeed: 1.8, lightRotation: 0.1, flowStrength: 0.9 } },
  ];
  const reverse = structuredClone(setup);
  reverse.assignments.idle.patterns.reverse();
  const a = step(new ExpressionPlayer(setup), 2),
    b = step(new ExpressionPlayer(reverse), 2);
  assert.deepEqual(a, b);
  assert.ok(Math.abs(a.u_lavaPhase[0] - a.u_lavaPhase[1]) < 1e-10);
  assert.ok(Math.abs(a.u_idleAmount - 1.3) < 1e-6);
});
test("two Stir profiles keep separate strengths and phases", () => {
  const setup = assignedSetup();
  setup.assignments.idle.patterns = [
    { kind: "stir", settings: { strength: 0.4, speed: 0.5, distortion: 0.2 } },
    { kind: "stir", settings: { strength: 0.8, speed: 2, distortion: 1.7 } },
  ];
  const frame = step(new ExpressionPlayer(setup), 1);
  assert.equal(frame.u_effectCount, 2);
  const [a, x, p, c, b, y, q, d] = frame["u_effects[0]"];
  assert.equal(a, 4);
  assert.equal(b, 4);
  assert.ok(Math.abs(y - 2 * x) < 1e-10);
  assert.ok(Math.abs(q - 4 * p) < 1e-10);
  assert.equal(c, 0.2);
  assert.equal(d, 1.7);
});
test("a comparison added during work receives held state and recent typing", () => {
  const p = new ExpressionPlayer(assignedSetup());
  step(p, 25);
  p.trigger("working");
  step(p, 1);
  p.trigger("typing");
  step(p, 0.04);
  const original = p.frame();
  const setup = assignedSetup();
  setup.assignments.typing = {
    id: "surface-react",
    name: "Ripple",
    description: "",
    patterns: [{ kind: "ripple", settings: { strength: 0.4 } }],
    duration: 0.24,
    color: { mode: "none", palette: "mono", strength: 0, attack: 0.1, duration: 0.3 },
  };
  const copy = p.fork(setup);
  assert.equal(copy.working, true);
  assert.equal(copy.time, p.time);
  assert.ok(copy.signals("typing").motion > 0);
  assert.ok(copy.frame().waveTexture.some((x) => x > 0));
  assert.deepEqual(copy.frame().u_lavaPhase, original.u_lavaPhase);
  assert.deepEqual(p.frame(), original);
});
test("zero-speed Lava settles without scheduling more draws; typing wakes it", () => {
  const setup = assignedSetup();
  Object.assign(setup.assignments.idle.patterns[0].settings, {
    flowSpeed: 0,
    lightRotation: 0,
  });
  const p = new ExpressionPlayer(setup);
  step(p, 3);
  assert.equal(p.needsAnimation(), false);
  p.trigger("typing");
  assert.equal(p.needsAnimation(), true);
  step(p, 5);
  assert.equal(p.needsAnimation(), false);
  p.trigger("hide");
  assert.equal(p.needsAnimation(), false);
});
test("effect parameters clamp only declared fields", () => {
  assert.deepEqual(readPattern("stir", { strength: 99, speed: -1, unrelated: 2 }), {
    kind: "stir", settings: { strength: 2.5, speed: 0 },
  });
  assert.deepEqual(readPattern("nudge", { strength: Number.NaN, speed: 1 }), { kind: "nudge", settings: {} });
});
