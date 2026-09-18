import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { variationPicker, variations, withLabStudies } =
  await import('./lab-studies.ts');
const { makeLabSetup } = await import('./lab-setup.ts');
const { validateSetup, eventConfig } = await import('./setup.ts');

test('variation is repeatable and reflects the configured weights', () => {
  const first = variationPicker(19),
    second = variationPicker(19);
  const chosen = Array.from({ length: 1000 }, () => first(variations.typing));
  assert.deepEqual(
    chosen,
    Array.from({ length: 1000 }, () => second(variations.typing)),
  );
  const sparks = chosen.filter((id) => id === 'nudge-sparks').length;
  assert.ok(sparks > 70 && sparks < 130);
});

test('new studies preserve edited drafts and support event assignments', () => {
  const setup = makeLabSetup();
  setup.expressions.find((e) => e.id === 'entrance-print').motions[0].duration =
    0.8;
  const draft = withLabStudies(setup);
  assert.deepEqual(draft, setup);
  draft.events.idle = 'swirl';
  draft.events.working = 'orbit';
  draft.events.typing = 'nudge-sparks';
  draft.events.flourish = 'entrance-fill-ripple';
  validateSetup(draft);
  const config = eventConfig(draft, 'flourish');
  assert.equal(config.appearanceTarget, 'avatar');
  assert.equal(config.entranceRipple, true);
  assert.equal(config.typingSparks, true);
  assert.equal(config.idleSwirl, 0.8);
  assert.equal(config.workingOrbit, 1);
});

test('event accents do not recolor other events or the idle baseline', () => {
  const setup = makeLabSetup();
  setup.events.submit = 'ember-sparks';
  assert.equal(eventConfig(setup).activityColor, 'opal');
  assert.equal(eventConfig(setup, 'typing').activityColor, 'opal');
  assert.equal(eventConfig(setup, 'submit').activityColor, 'ember');
});
