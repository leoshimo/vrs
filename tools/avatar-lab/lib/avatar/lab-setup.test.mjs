import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { makeLabSetup, readLabDraft } = await import('./lab-setup.ts');
const { validateSetup } = await import('./setup.ts');

test('browser drafts retain tuning and event assignments', () => {
  const setup = makeLabSetup();
  validateSetup(setup);
  setup.events.typing = 'pressure-edge-press';
  setup.appearance.pitch = 2.4;
  const restored = readLabDraft(JSON.stringify(setup));
  assert.deepEqual(restored, setup);
});

test('missing, malformed, and incompatible drafts use code defaults', () => {
  for (const raw of [null, '{', '{}', '{"version":2}']) {
    assert.deepEqual(readLabDraft(raw), makeLabSetup());
  }
  const setup = makeLabSetup();
  setup.events.typing = 'missing';
  assert.deepEqual(readLabDraft(JSON.stringify(setup)), makeLabSetup());
});
