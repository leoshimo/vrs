import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { PlaybackClock } = await import('./playback.ts');
const { preset, sequencePlan, paintCell, cellAt } =
  await import('./sequence.ts');
const { expressionGroups } = await import('./expression-groups.ts');
const { makeWorkspace, assignmentNames } = await import('./workspace.ts');
test('presets distinguish action dismissal from page completion and overlap Submit with Working', () => {
  const action = sequencePlan(preset('action')),
    page = sequencePlan(preset('page'));
  const submitted = action.steps.find((s) => s.action === 'submit');
  assert.equal(
    action.steps.find((s) => s.action === 'working').at,
    submitted.at,
  );
  assert.ok(action.steps.some((s) => s.action === 'hide' && s.at > 9000));
  assert.ok(!page.steps.some((s) => s.action === 'hide' && s.at > 9000));
});
test('held cells merge into intervals; entrances are properties of their opening edge', () => {
  let s = preset('idle');
  s = paintCell(s, 'window', cellAt(2), false);
  s.entrances[cellAt(2.25)] = 'openAfterIdle';
  const steps = sequencePlan(s).steps;
  assert.deepEqual(
    steps.filter((s) => s.action === 'hide').map((s) => s.at),
    [0, 2000],
  );
  assert.equal(steps.find((s) => s.action === 'openAfterIdle').at, 2250);
  const shorter = sequencePlan({ ...s, duration: 2 });
  assert.ok(shorter.steps.every((s) => s.at < 2000));
});
test('repeated submissions overlap, with loading starting on the last tap', () => {
  const plan = sequencePlan(preset('repeated'));
  const times = plan.steps
    .filter((s) => s.action === 'submit')
    .map((s) => s.at);
  assert.deepEqual(times, [4500, 4750, 5000]);
  assert.equal(plan.steps.find((s) => s.action === 'working').at, times.at(-1));
  assert.ok(plan.steps.find((s) => s.action === 'complete').at > times.at(-1));
});
test('quick actions submit at the closing edge without entering Working', () => {
  const plan = sequencePlan(preset('quick'));
  const index = plan.steps.findIndex((s) => s.action === 'submit');
  assert.deepEqual(plan.steps.slice(index, index + 2), [
    { at: 4000, action: 'submit' },
    { at: 4000, action: 'hide' },
  ]);
  assert.ok(
    !plan.steps.some((s) => s.action === 'working' || s.action === 'complete'),
  );
});
test('clock pauses, resumes and loops without dropping or duplicating events', () => {
  const plan = sequencePlan(preset('action')),
    clock = new PlaybackClock(plan);
  clock.restart();
  const first = clock.advance(1200, true);
  assert.equal(first.filter((s) => s.action === 'openAfterIdle').length, 1);
  clock.running = false;
  assert.deepEqual(clock.advance(5000, true), []);
  assert.equal(clock.elapsed, 1200);
  clock.running = true;
  const middle = clock.advance(8000, true);
  assert.equal(middle.filter((s) => s.action === 'submit').length, 1);
  const end = clock.advance(2800, true);
  assert.deepEqual(
    end.slice(-2).map((s) => s.action),
    ['hide', 'idle'],
  );
  assert.equal(clock.elapsed, 0);
  assert.deepEqual(clock.advance(1200, false), first.slice(2));
});
test('all catalog effects appear exactly once in every assignment grouping', () => {
  const setup = makeWorkspace();
  for (const a of assignmentNames)
    assert.deepEqual(
      expressionGroups(setup.expressions, a)
        .flatMap((g) => g.expressions.map((e) => e.id))
        .sort(),
      setup.expressions.map((e) => e.id).sort(),
    );
});
