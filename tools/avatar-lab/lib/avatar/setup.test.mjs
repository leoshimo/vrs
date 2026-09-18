import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { makeDefaultSetup, compose, expression, validateSetup } =
  await import('./setup.ts');
const { createSignalMotion, advanceSignalMotion } =
  await import('./signal-motion.ts');
const { withTypingStudies, typingStudies } =
  await import('./typing-studies.ts');
test('typing studies retain saved edits and can be assigned and saved', () => {
  const original = makeDefaultSetup();
  const before = JSON.stringify(original);
  const setup = withTypingStudies(original);
  assert.equal(JSON.stringify(original), before);
  assert.deepEqual(setup.events, original.events);
  const edited = structuredClone(setup);
  edited.expressions.find(
    (e) => e.id === 'pressure-dimple',
  ).motions[0].settings.typingEnergy = 0.2;
  const loaded = withTypingStudies(edited);
  assert.equal(loaded.expressions.length, setup.expressions.length);
  assert.equal(
    loaded.expressions.find((e) => e.id === 'pressure-dimple').motions[0]
      .settings.typingEnergy,
    0.2,
  );
  for (const study of typingStudies) {
    setup.events.typing = study.id;
    const saved = JSON.parse(JSON.stringify(setup));
    validateSetup(saved);
    const config = compose(
      saved.appearance,
      expression(saved, 'typing').motions,
    );
    assert.equal(config.typingMotion, 'pressure');
    assert.equal(
      config.pressureSpread,
      study.motions[0].settings.pressureSpread,
    );
    assert.equal(
      config.pressureOffset,
      study.motions[0].settings.pressureOffset,
    );
  }
});
test('saved setup round-trips the full library and event assignments', () => {
  const setup = JSON.parse(JSON.stringify(makeDefaultSetup()));
  validateSetup(setup);
  assert.equal(expression(setup, 'typing').name, 'Nudge');
  assert.equal(expression(setup, 'submit').name, 'Sparks');
  assert.ok(setup.expressions.some((e) => e.name === 'Tilt'));
  assert.ok(setup.expressions.some((e) => e.name === 'Echo'));
  assert.equal(expression(setup, 'working').motions.length, 2);
  assert.notEqual(setup.events.open, setup.events.flourish);
});
test('Stir and Ripple retain independent settings and do not alter saved sources', () => {
  const s = makeDefaultSetup(),
    before = JSON.stringify(s);
  const m = expression(s, 'working').motions;
  const config = compose(s.appearance, [
    ...expression(s, 'idle').motions,
    ...m,
    ...expression(s, 'typing').motions,
    ...expression(s, 'submit').motions,
  ]);
  assert.equal(config.effectMix.agitation, 1);
  assert.equal(config.effectMix.wave, 0);
  assert.equal(config.effectLayerSettings.agitation.loadingStrength, 1.6);
  assert.equal(config.effectLayerSettings.agitation.loadingRate, 1.4);
  assert.equal(config.loopRipple.surfaceAmplitude, 0.28);
  assert.equal(config.loopRipple.surfaceDuration, 5);
  assert.equal(config.typingMotion, 'nudge');
  assert.equal(config.avatarDispatch, 'sparks');
  assert.equal(JSON.stringify(s), before);
});
test('assignments cannot reference missing expressions or incompatible actions', () => {
  const s = makeDefaultSetup();
  s.events.working = 'sparks';
  assert.throws(() => validateSetup(s), /Working assignment/);
  s.events.working = 'missing';
  assert.throws(() => validateSetup(s), /Working assignment/);
});
test('invalid or duplicated motion families are rejected before saving', () => {
  const s = makeDefaultSetup(),
    e = expression(s, 'working');
  e.motions.push(structuredClone(e.motions[0]));
  assert.throws(() => validateSetup(s), /one motion/);
  e.motions.pop();
  e.motions[0].settings.loadingRate = Infinity;
  assert.throws(() => validateSetup(s), /setting value/);
});
test('typing and submit retain sustained Working until explicitly stopped', () => {
  const m = createSignalMotion();
  for (let i = 0; i < 180; i++)
    advanceSignalMotion(m, 'idle', 0, true, 1 / 60, true, { working: true });
  assert.ok(m.loading.value > 0.99);
  advanceSignalMotion(m, 'typing', 1, true, 1 / 60, true, {
    working: true,
    typingNudge: true,
  });
  assert.ok(m.fieldImpulseVelocity > 0);
  assert.ok(m.loading.value > 0.99);
  advanceSignalMotion(m, 'dispatch', 2, true, 1 / 60, true, { working: true });
  assert.ok(m.loading.value > 0.99);
  for (let i = 0; i < 180; i++)
    advanceSignalMotion(m, 'idle', 2, true, 1 / 60, true, { working: false });
  assert.ok(m.loading.value < 0.001);
});

test('a sustained Stir cannot overwrite a one-shot Ripple profile', () => {
  const s = makeDefaultSetup();
  const ripple = s.expressions.find((e) => e.id === 'surface-react').motions[0];
  const stir = expression(s, 'working').motions[0];
  const c = compose(s.appearance, [stir, ripple]);
  assert.equal(c.surfaceDuration, ripple.settings.surfaceDuration);
  assert.equal(c.surfaceAmplitude, ripple.settings.surfaceAmplitude);
  assert.equal(c.loadingStrength, stir.settings.loadingStrength);
});
