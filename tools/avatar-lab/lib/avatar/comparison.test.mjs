import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { makeWorkspace, readWorkspace } = await import('./workspace.ts');
const { comparisonSetup, assignmentComparisons, defaultInput } =
  await import('./comparison.ts');
const { ExpressionPlayer } =
  await import('../../../../vrsjmp/src/avatar/expression-player.ts');
const advance = (p, seconds) => {
  for (let i = 0; i < seconds * 120; i++) p.frame(1 / 120);
};

test('comparisons isolate one expression and leave the assigned setup intact', () => {
  const setup = makeWorkspace();
  const assignments = structuredClone(setup.assignments);
  const dimple = setup.expressions.find((e) => e.id === 'pressure-dimple');
  assert.equal(defaultInput(dimple), 'typing');
  const preview = new ExpressionPlayer(
    comparisonSetup(setup, { id: dimple.id, input: 'typing' }),
  );
  advance(preview, 0.4);
  assert.equal(preview.signals().motion, 0);
  preview.trigger('submit');
  advance(preview, 0.08);
  assert.equal(preview.signals().motion, 0);
  preview.trigger('typing');
  advance(preview, 0.08);
  assert.ok(preview.signals().motion > 0.1);
  advance(preview, 0.5);
  assert.equal(preview.signals().motion, 0);
  const held = new ExpressionPlayer(
    comparisonSetup(setup, { id: dimple.id, input: 'idle' }),
  );
  advance(held, 0.6);
  assert.ok(held.signals().motion > 0.99);
  assert.deepEqual(setup.assignments, assignments);
  for (const p of assignmentComparisons(setup)) {
    assert.equal(
      Object.values(comparisonSetup(setup, p).assignments).filter(Boolean)
        .length,
      1,
    );
  }
});
test('the proposal enables only Working color; expressions can independently opt in afterward', () => {
  const old = makeWorkspace();
  old.appearance.activityColor = 'spectrum';
  old.assignments.open = 'gather';
  old.expressions.forEach((e) => (e.color.mode = 'accent'));
  const next = readWorkspace(null, null, null, JSON.stringify(old));
  assert.deepEqual(next.assignments, old.assignments);
  assert.equal(next.appearance.activityColor, 'spectrum');
  assert.deepEqual(
    next.expressions.filter((e) => e.color.mode !== 'none').map((e) => e.id),
    [next.assignments.working],
  );
  const player = new ExpressionPlayer(next);
  advance(player, 1);
  assert.equal(player.signals().color, 0);
  player.trigger('typing');
  advance(player, 0.1);
  assert.equal(player.signals().color, 0);
  player.trigger('working');
  advance(player, 0.5);
  assert.ok(player.signals().color > 0.1);
  player.trigger('idle');
  advance(player, 2);
  assert.ok(player.signals().color < 0.0001);
  next.expressions.find((e) => e.id === next.assignments.typing).color.mode =
    'accent';
  const saved = readWorkspace(JSON.stringify(next));
  assert.equal(
    saved.expressions.find((e) => e.id === next.assignments.typing).color.mode,
    'accent',
  );
  const updated = new ExpressionPlayer(saved);
  updated.trigger('typing');
  advance(updated, 0.08);
  assert.ok(updated.signals().color > 0.1);
});
