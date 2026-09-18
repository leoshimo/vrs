import { test } from "node:test";
import assert from "node:assert/strict";
import { register } from "node:module";
register("./test-resolve.mjs", import.meta.url);
const { ResponseCurve, advanceMomentum } = await import("./response-curve.ts");

test("two interactions form one additive curve without altering earlier samples", () => {
  const first = new ResponseCurve(),
    second = new ResponseCurve(),
    combined = new ResponseCurve();
  first.add(1, 0.3);
  second.add(1.15, 0.3, 0.7);
  combined.add(1, 0.3);
  const past = combined.sample(1.1);
  combined.add(1.15, 0.3, 0.7);
  assert.equal(combined.sample(1.1), past);
  for (let time = 0.9; time < 1.6; time += 0.003)
    assert.ok(Math.abs(combined.sample(time) - first.sample(time) - second.sample(time)) < 1e-12);
  assert.equal(combined.sample(0.9), 0);
  assert.equal(combined.sample(1.6), 0);
});
test("delayed sampling retains two separated traveling peaks", () => {
  const curve = new ResponseCurve();
  curve.add(0, 0.12);
  curve.add(0.25, 0.12);
  assert.ok(curve.sample(0.03) > 0.7);
  assert.ok(curve.sample(0.28) > 0.7);
  assert.equal(curve.sample(0.18), 0);
  const time = 0.6;
  assert.ok(curve.sample(time - 0.57) > 0.7);
  assert.ok(curve.sample(time - 0.32) > 0.7);
});
test("ring buffer reuses expired samples without reviving old signals", () => {
  const curve = new ResponseCurve();
  curve.add(0, 0.2);
  curve.add(40, 0.2, 0.5);
  assert.equal(curve.sample(39.8), 0);
  assert.ok(curve.sample(40.05) > 0);
  assert.equal(curve.sample(41), 0);
});
test("momentum has a frame-rate independent speed limit and continuous phase", () => {
  const a = { speed: 0, phase: 0 },
    b = { speed: 0, phase: 0 };
  for (let i = 0; i < 120; i++) advanceMomentum(a, 24, 1 / 120);
  for (let i = 0; i < 30; i++) advanceMomentum(b, 24, 1 / 30);
  assert.ok(Math.abs(a.speed - b.speed) < 1e-12);
  assert.ok(Math.abs(a.phase - b.phase) < 1e-12);
  assert.ok(a.speed > 0 && a.speed < 2.4);
  const phase = a.phase;
  advanceMomentum(a, 0, 0.3);
  assert.ok(a.phase > phase);
  assert.ok(a.speed < b.speed);
});
