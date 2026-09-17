import { test } from "node:test";
import assert from "node:assert/strict";
import { createSignalMotion, advanceSignalMotion } from "./signal-motion.ts";
test("an interrupted state retains its pose and velocity", () => {
  const m = createSignalMotion();
  for (let i = 0; i < 16; i++) advanceSignalMotion(m, "loading", 0, true, 1 / 60);
  const before = structuredClone(m);
  advanceSignalMotion(m, "dispatch", 0, true, 0);
  assert.deepEqual(m, before);
  advanceSignalMotion(m, "dispatch", 0, true, 1 / 60);
  assert.ok(Math.abs(m.loading.value - before.loading.value) < 0.08);
  assert.ok(m.phase >= before.phase);
});
test("rapid typing and state reversals stay finite and settle", () => {
  const m = createSignalMotion();
  const modes = ["idle", "typing", "loading", "dispatch"];
  for (let i = 0; i < 1500; i++) {
    advanceSignalMotion(m, modes[i % 4], i, true, 1 / 60);
    assert.ok(Number.isFinite(m.phase));
    for (const k of ["typing", "loading", "dispatch"]) assert.ok(Math.abs(m[k].value) < 2);
  }
  for (let i = 0; i < 600; i++) advanceSignalMotion(m, "idle", 1500, false, 1 / 60);
  assert.ok(
    m.typing.value < 0.001 &&
      m.loading.value < 0.001 &&
      m.dispatch.value < 0.001 &&
      m.speed.value < 0.001,
  );
});
test("reduced motion resolves directly without accumulating rotation", () => {
  const m = createSignalMotion();
  advanceSignalMotion(m, "dispatch", 1, true, 1, false);
  assert.equal(m.dispatch.value, 1);
  assert.equal(m.phase, 0);
  assert.equal(m.speed.value, 0);
});
test("loading is smooth and only moderately faster than idle", () => {
  const idle = createSignalMotion(),
    loading = createSignalMotion();
  for (let i = 0; i < 240; i++) {
    advanceSignalMotion(idle, "idle", 0, true, 1 / 60);
    advanceSignalMotion(loading, "loading", 0, true, 1 / 60);
  }
  assert.ok(loading.speed.value > idle.speed.value * 1.4);
  assert.ok(loading.speed.value < idle.speed.value * 1.8);
  const speed = loading.speed.value;
  advanceSignalMotion(loading, "loading", 0, true, 1 / 60);
  assert.ok(Math.abs(loading.speed.value - speed) < 0.001);
  assert.equal(loading.burst.value, 0);
});
test("one typing event flicks immediately and coasts to idle even when typing stays selected", () => {
  const m = createSignalMotion();
  for (let i = 0; i < 180; i++) advanceSignalMotion(m, "idle", 0, true, 1 / 60);
  const before = m.phase;
  advanceSignalMotion(m, "typing", 1, true, 1 / 60);
  assert.ok(m.phase - before > 0.06, "the first frame must carry the attack");
  let prior = m.flickVelocity;
  for (let i = 0; i < 90; i++) {
    advanceSignalMotion(m, "typing", 1, true, 1 / 60);
    assert.ok(m.flickVelocity <= prior);
    prior = m.flickVelocity;
  }
  assert.ok(m.flickVelocity < 0.001 && m.typing.value < 0.001 && m.tap.value < 0.001);
  assert.ok(Math.abs(m.speed.value - 0.18) < 0.001);
  const pose = m.phase;
  advanceSignalMotion(m, "typing", 2, true, 0);
  assert.equal(m.phase, pose);
  advanceSignalMotion(m, "typing", 2, true, 1 / 60);
  assert.ok(m.flickVelocity > 3.5, "another keystroke must flick again");
});
test("dispatch fires once, keeps continuity on interruption, and settles", () => {
  const m = createSignalMotion();
  let peak = 0;
  for (let i = 0; i < 12; i++) {
    advanceSignalMotion(m, "dispatch", 1, true, 1 / 60);
    peak = Math.max(peak, m.burst.value);
  }
  assert.ok(peak > 0.9);
  const before = structuredClone(m);
  advanceSignalMotion(m, "typing", 2, true, 0);
  assert.deepEqual(m, before);
  advanceSignalMotion(m, "typing", 2, true, 1 / 60);
  assert.ok(Math.abs(m.burst.value - before.burst.value) < 0.15);
  for (let i = 0; i < 240; i++) advanceSignalMotion(m, "dispatch", 3, true, 1 / 60);
  assert.ok(m.burst.value < 0.001 && m.echo.value < 0.001);
  advanceSignalMotion(m, "dispatch", 4, true, 1 / 60);
  assert.ok(m.burst.value > 0.1);
});
test("typing varies direction per event, stays bounded, and settles without spinning the volume", () => {
  const m = createSignalMotion(42);
  const directions = new Set();
  let peak = 0;
  for (let pulse = 1; pulse <= 100; pulse++) {
    advanceSignalMotion(m, "typing", pulse, true, 1 / 60);
    directions.add(Math.floor(Math.atan2(m.pokeY.velocity, m.pokeX.velocity) * 4));
    peak = Math.max(peak, Math.hypot(m.pokeX.value, m.pokeY.value));
    assert.ok(Math.hypot(m.pokeX.velocity, m.pokeY.velocity) <= 28);
  }
  assert.ok(directions.size > 8, "repeated input should not follow one direction");
  assert.ok(peak > 0.15 && peak < 1, "visible attack without runaway deformation");
  const idle = createSignalMotion(42);
  for (let i = 0; i < 100; i++) advanceSignalMotion(idle, "idle", 0, true, 1 / 60);
  assert.ok(
    Math.abs(m.drift - idle.drift) < 0.00001,
    "typing must not accelerate the volume lighting",
  );
  assert.ok(Math.hypot(m.poke2X.value, m.poke2Y.value) > 0, "crosscurrents have a second impulse");
  const before = structuredClone(m);
  advanceSignalMotion(m, "typing", 101, true, 0);
  assert.deepEqual(m, before);
  for (let i = 0; i < 180; i++) advanceSignalMotion(m, "idle", 100, true, 1 / 60);
  assert.ok(Math.hypot(m.pokeX.value, m.pokeY.value) < 0.001);
  assert.ok(Math.hypot(m.poke2X.value, m.poke2Y.value) < 0.001);
});
test("a second dispatch keeps the previous outward ripple alive", () => {
  const m = createSignalMotion(42);
  for (let i = 0; i < 18; i++) advanceSignalMotion(m, "dispatch", 1, true, 1 / 60);
  const age = m.releaseAge;
  advanceSignalMotion(m, "dispatch", 2, true, 1 / 60);
  assert.equal(m.releaseAge, 0);
  assert.ok(m.priorReleaseAge > age);
  for (let i = 0; i < 90; i++) advanceSignalMotion(m, "idle", 2, true, 1 / 60);
  assert.ok(m.releaseAge > 0.95 && m.priorReleaseAge > 0.95);
  advanceSignalMotion(m, "typing", 3, true, 1 / 60, false);
  assert.equal(m.pokeX.value, 0);
  assert.equal(m.pokeY.value, 0);
  assert.ok(m.releaseAge > 0.95);
});
test("invocation wakes the interior without resetting its pose, then settles", () => {
  const m = createSignalMotion(42);
  for (let i = 0; i < 120; i++) advanceSignalMotion(m, "idle", 0, true, 1 / 60);
  const before = structuredClone(m);
  advanceSignalMotion(m, "idle", 0, true, 0, true, { invocation: 1 });
  assert.deepEqual(m, before, "a redraw must not consume the wake event");
  let peak = 0;
  for (let i = 0; i < 90; i++) {
    const old = m.drift;
    advanceSignalMotion(m, "idle", 0, true, 1 / 60, true, { invocation: 1 });
    assert.ok(m.drift >= old && m.drift - old < 0.03, "continuous motion without a phase jump");
    peak = Math.max(peak, m.fieldSpeed.value);
  }
  assert.ok(peak > before.fieldSpeed.value * 1.7, "invocation visibly energizes idle");
  for (let i = 0; i < 480; i++)
    advanceSignalMotion(m, "idle", 0, true, 1 / 60, true, { invocation: 1 });
  assert.ok(m.wake.value < 0.001);
  assert.ok(Math.abs(m.fieldSpeed.value - 0.38) < 0.001);
});
test("drift moves one way and slows to rest without resetting its offset", () => {
  const m = createSignalMotion(42);
  for (let i = 0; i < 180; i++) {
    const offset = m.transport;
    advanceSignalMotion(m, "loading", 0, true, 1 / 60, true, {
      loading: "drift",
    });
    assert.ok(m.transport >= offset);
  }
  assert.ok(m.transport > 2);
  const atRelease = m.transport;
  for (let i = 0; i < 240; i++)
    advanceSignalMotion(m, "idle", 0, true, 1 / 60, true, { loading: "drift" });
  assert.ok(m.transport > atRelease, "the field coasts rather than snapping to origin");
  const settled = m.transport;
  advanceSignalMotion(m, "idle", 0, true, 1 / 60, true, { loading: "drift" });
  assert.ok(Math.abs(m.transport - settled) < 0.00001);
});
test("quicker idle and agitation have distinct field speeds, and reduced motion stops them", () => {
  const modes = ["normal", "quicker", "agitated"];
  const speeds = modes.map((loading) => {
    const m = createSignalMotion(42);
    for (let i = 0; i < 300; i++)
      advanceSignalMotion(m, "loading", 0, true, 1 / 60, true, { loading });
    return m.fieldSpeed.value;
  });
  assert.ok(speeds[1] > speeds[2] && speeds[2] > speeds[0]);
  const m = createSignalMotion(42);
  advanceSignalMotion(m, "loading", 0, true, 1 / 60, true, {
    loading: "drift",
    invocation: 1,
  });
  advanceSignalMotion(m, "loading", 0, true, 1 / 60, false, {
    loading: "drift",
    invocation: 1,
  });
  assert.equal(m.wake.value, 0);
  assert.equal(m.fieldSpeed.value, 0);
  assert.equal(m.transport, 0);
});
test("stronger impulses keep continuity and a longer tail outlives dispatch", () => {
  const calm = createSignalMotion(42),
    strong = createSignalMotion(42);
  for (let i = 0; i < 12; i++) {
    advanceSignalMotion(calm, "dispatch", 1, true, 1 / 60, true, {
      dispatchEnergy: 0.6,
      settle: 0.7,
    });
    advanceSignalMotion(strong, "dispatch", 1, true, 1 / 60, true, {
      dispatchEnergy: 2,
      settle: 2.4,
    });
  }
  assert.ok(strong.burst.value > calm.burst.value * 2);
  for (let i = 0; i < 120; i++) {
    advanceSignalMotion(calm, "idle", 1, true, 1 / 60, true, { settle: 0.7 });
    advanceSignalMotion(strong, "idle", 1, true, 1 / 60, true, { settle: 2.4 });
  }
  assert.ok(strong.burst.value < 0.001, "short dispatch has resolved");
  assert.ok(strong.momentum.value > 0.1, "the interior keeps coasting");
  assert.ok(calm.momentum.value < 0.005, "short settling is distinct");
  for (let i = 0; i < 900; i++)
    advanceSignalMotion(strong, "idle", 1, true, 1 / 60, true, { settle: 2.4 });
  assert.ok(strong.momentum.value < 0.001);
});
test("maximum typing energy stays bounded through a rapid key repeat", () => {
  const m = createSignalMotion(5);
  for (let i = 1; i < 600; i++) {
    advanceSignalMotion(m, "typing", i, true, 1 / 60, true, {
      typingEnergy: 2.5,
    });
    assert.ok(Math.hypot(m.pokeX.value, m.pokeY.value) < 1.2);
    assert.ok(Math.hypot(m.pokeX.velocity, m.pokeY.velocity) < 25);
  }
  advanceSignalMotion(m, "idle", 600, true, 1 / 60, false);
  assert.equal(m.momentum.value, 0);
});
test("working speed control changes travel without an event-time discontinuity", () => {
  const slow = createSignalMotion(7),
    fast = createSignalMotion(7);
  for (let i = 0; i < 240; i++) {
    advanceSignalMotion(slow, "loading", 0, true, 1 / 60, true, {
      loading: "drift",
      loadingRate: 0.5,
    });
    advanceSignalMotion(fast, "loading", 0, true, 1 / 60, true, {
      loading: "drift",
      loadingRate: 2,
    });
  }
  assert.ok(fast.transport > slow.transport * 2);
  const snapshot = structuredClone(fast);
  advanceSignalMotion(fast, "loading", 0, true, 0, true, {
    loading: "drift",
    loadingRate: 0.5,
  });
  assert.deepEqual(fast, snapshot);
});
test("a soft nudge has a smaller material impulse than an agitation kick", () => {
  const soft = createSignalMotion(11),
    hard = createSignalMotion(11);
  advanceSignalMotion(soft, "typing", 1, true, 1 / 60, true, {
    typingEnergy: 0.15,
  });
  advanceSignalMotion(hard, "typing", 1, true, 1 / 60, true, {
    typingEnergy: 2,
  });
  assert.ok(hard.tap.value > soft.tap.value * 2);
  for (let i = 0; i < 500; i++) advanceSignalMotion(hard, "idle", 1, true, 1 / 60);
  assert.ok(hard.tap.value < 0.001);
});

test("field nudge advances the existing clock and settles without changing the light-kick springs", () => {
  const control = createSignalMotion(42),
    nudged = createSignalMotion(42);
  for (let i = 0; i < 120; i++) {
    advanceSignalMotion(control, "idle", 0, true, 1 / 60);
    advanceSignalMotion(nudged, "idle", 0, true, 1 / 60);
  }
  const before = nudged.drift;
  advanceSignalMotion(nudged, "typing", 1, true, 0, true, {
    typingNudge: true,
  });
  assert.equal(nudged.drift, before);
  for (let i = 0; i < 360; i++) {
    advanceSignalMotion(control, "idle", 0, true, 1 / 60);
    advanceSignalMotion(nudged, "typing", 1, true, 1 / 60, true, {
      typingNudge: true,
    });
    assert.ok(nudged.drift >= before);
    assert.equal(nudged.pokeX.value, 0);
    assert.equal(nudged.pokeY.value, 0);
  }
  assert.ok(nudged.drift > control.drift + 0.1);
  assert.ok(nudged.drift < control.drift + 0.25);
  assert.ok(nudged.fieldImpulseVelocity < 1e-8);
  assert.ok(Math.abs(nudged.fieldSpeed.value - control.fieldSpeed.value) < 1e-8);
});
