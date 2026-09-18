import { test } from "node:test";
import assert from "node:assert/strict";
import { register } from "node:module";
register("./test-resolve.mjs", import.meta.url);
const { makeDefaultSetup, eventConfig } = await import("./setup.ts");
const { createSignalMotion, advanceSignalMotion } = await import("./signal-motion.ts");

test("Working ends without changing profiles or triggering a release ripple", () => {
  const setup = makeDefaultSetup(),
    config = eventConfig(setup);
  const motion = createSignalMotion(42);
  for (let i = 0; i < 180; i++)
    advanceSignalMotion(motion, "idle", 0, true, 1 / 60, true, { working: true });
  const before = motion.loading.value,
    age = motion.releaseAge;
  advanceSignalMotion(motion, "idle", 0, true, 1 / 60, true, { working: false });
  assert.ok(motion.loading.value < before && motion.loading.value > before - 0.01);
  assert.ok(motion.releaseAge > age);
  assert.equal(motion.burst.value, 0);
  assert.equal(motion.dispatch.value, 0);
  assert.deepEqual(eventConfig(setup), config);
  assert.equal(config.effectMix.agitation, 1);
  assert.equal(config.effectMix.wave, 0);
  assert.equal(config.loopRipple.surfaceAmplitude, 0.28);
  for (let i = 0; i < 150; i++)
    advanceSignalMotion(motion, "idle", 0, true, 1 / 60, true, { working: false });
  assert.ok(motion.loading.value < 0.001);
  assert.equal(motion.burst.value, 0);
});

test("Nudge has an immediate small kick and loses its tail within 400ms", () => {
  const control = createSignalMotion(42),
    typed = createSignalMotion(42);
  const activity = { typingNudge: true, typingEnergy: 0.6 };
  advanceSignalMotion(control, "idle", 0, true, 1 / 60);
  advanceSignalMotion(typed, "typing", 1, true, 1 / 60, true, activity);
  assert.ok(typed.drift - control.drift > 0.015);
  assert.ok(typed.drift - control.drift < 0.025);
  for (let i = 0; i < 24; i++) {
    advanceSignalMotion(control, "idle", 0, true, 1 / 60);
    advanceSignalMotion(typed, "idle", 1, true, 1 / 60, true, activity);
  }
  assert.ok(typed.fieldImpulseVelocity < 0.01);
  assert.ok(typed.drift - control.drift < 0.12);
  for (let i = 0; i < 120; i++)
    advanceSignalMotion(typed, "typing", i + 2, true, 1 / 120, true, activity);
  assert.ok(typed.fieldImpulseVelocity <= 3);
});

test("typing sparks are brief and do not trigger a dispatch or surface ripple", () => {
  const motion = createSignalMotion(42);
  const activity = { typingNudge: true, typingSparks: true, typingEnergy: 0.45 };
  advanceSignalMotion(motion, "typing", 1, true, 1 / 60, true, activity);
  assert.equal(motion.typingSparkAge, 0);
  assert.equal(motion.burst.value, 0);
  assert.ok(motion.releaseAge > 10);
  assert.equal(motion.pokeX.value, 0);
  for (let i = 0; i < 20; i++) advanceSignalMotion(motion, "idle", 1, true, 1 / 60, true, activity);
  assert.ok(motion.typingSparkAge > 0.24);
  assert.ok(motion.fieldImpulseVelocity < 0.03);
});
