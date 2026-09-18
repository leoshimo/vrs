import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { makeDefaultSetup, expression } = await import('./setup.ts');
const { groupedLibrary, workingEntry, motionAvailability, variantName } =
  await import('./expression-library.ts');

test('library groups every single-motion expression once and retains its variants', () => {
  const setup = makeDefaultSetup();
  const groups = groupedLibrary(setup);
  assert.equal(groups.length, 17);
  assert.equal(new Set(groups.map((g) => g.id)).size, groups.length);
  assert.deepEqual(
    groups.flatMap((g) => g.entries.map((e) => e.id)).sort(),
    setup.expressions
      .filter((e) => e.motions.length === 1)
      .map((e) => e.id)
      .sort(),
  );
  assert.deepEqual(
    groups.find((g) => g.id === 'surface').entries.map(variantName),
    ['Short reaction', 'Repeating', 'Flourish'],
  );
  assert.equal(groups.find((g) => g.id === 'wave').entries.length, 6);
  assert.ok(
    !groups.some((g) =>
      g.entries.some((e) => e.id === 'working' || e.id === 'quick-open'),
    ),
  );
});

test('motion picker uses saved library settings and reports event compatibility', () => {
  const setup = makeDefaultSetup();
  setup.expressions.find(
    (e) => e.id === 'restless',
  ).motions[0].settings.loadingRate = 2.45;
  const groups = groupedLibrary(setup);
  const stir = groups.find((g) => g.id === 'agitation');
  assert.equal(workingEntry(stir).motions[0].settings.loadingRate, 2.45);
  const selected = expression(setup, 'working');
  const status = Object.fromEntries(
    groups.map((g) => [g.id, motionAvailability(g, selected)]),
  );
  assert.equal(status.agitation, 'Added');
  assert.equal(status.surface, 'Added');
  assert.equal(status.drift, 'Working');
  assert.equal(status.nudge, 'Typing');
  assert.equal(status.sparks, 'Submit');
  assert.equal(status.entrance, 'Open');
  assert.equal(workingEntry(groups.find((g) => g.id === 'sparks')), undefined);
});
