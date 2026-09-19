import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { makeWorkspace } = await import('./workspace.ts');
const { comparisonSetup } = await import('./comparison.ts');
const { ExpressionPlayer } =
  await import('../../../../vrsjmp/src/avatar/expression-player.ts');
const step = (p, seconds) => {
  for (let i = 0; i < seconds * 120; i++) p.advance(1 / 120);
};
test('comparison replaces only the chosen assignment and follows shared inputs', () => {
  const setup = makeWorkspace(),
    before = structuredClone(setup);
  const drift = setup.expressions.find((e) => e.id === 'drift');
  const preview = comparisonSetup(setup, 'working', drift);
  assert.deepEqual(preview.assignments, {
    ...setup.assignments,
    working: drift,
  });
  const p = new ExpressionPlayer(preview);
  step(p, 0.5);
  assert.equal(p.signals('working').motion, 0);
  p.trigger('working');
  step(p, 0.3);
  assert.ok(p.frame().u_printOffset[0] > 0);
  p.trigger('typing');
  step(p, 0.05);
  assert.ok(p.signals('typing').motion > 0);
  p.trigger('hide');
  assert.equal(p.visible, false);
  p.trigger('openAfterIdle');
  assert.equal(p.frame().u_appearance, 0);
  assert.deepEqual(setup, before);
});
