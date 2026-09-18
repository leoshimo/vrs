import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { sequencePlan, repeatPlan } = await import('./playback.ts');
const { expressionGroups } = await import('./expression-groups.ts');
const { makeWorkspace, assignmentNames } = await import('./workspace.ts');
const { ExpressionPlayer } =
  await import('../../../../vrsjmp/src/avatar/expression-player.ts');

test('sequence hides before each entrance and can run successive cycles', () => {
  const player = new ExpressionPlayer(makeWorkspace());
  const plan = sequencePlan();
  for (let cycle = 0; cycle < 2; cycle++) {
    let elapsed = 0;
    for (const step of plan.steps) {
      while (elapsed < step.at) {
        player.frame(1 / 120);
        elapsed += 1000 / 120;
      }
      if (step.action === 'open' || step.action === 'openAfterIdle')
        assert.equal(player.visible, false);
      player.trigger(step.action);
      if (step.action === 'openAfterIdle') {
        assert.ok(step.at >= 400);
        assert.equal(player.frame(0).u_appearance, 0);
        player.frame(0.03);
        assert.ok(
          player.frame(0).u_appearance > 0 && player.frame(0).u_appearance < 1,
        );
      }
    }
    assert.equal(player.visible, false);
    assert.equal(player.working, false);
    assert.ok(plan.duration > elapsed);
  }
});
test('repeat leaves time to see an entrance and submits two overlapping impulses', () => {
  const setup = makeWorkspace();
  const entrance = setup.expressions.find(
    (e) => e.id === setup.assignments.openAfterIdle,
  );
  const plan = repeatPlan('openAfterIdle', entrance);
  assert.deepEqual(
    plan.steps.map((s) => s.action),
    ['hide', 'openAfterIdle', 'hide'],
  );
  assert.ok(plan.steps[2].at - plan.steps[1].at >= entrance.duration * 1000);
  const submit = repeatPlan(
    'submit',
    setup.expressions.find((e) => e.id === setup.assignments.submit),
  );
  assert.equal(submit.steps.length, 2);
  assert.ok(submit.steps[1].at < 400);
  assert.ok(submit.duration > submit.steps[1].at + 500);
});
test('grouping retains every variant exactly once for every assignment', () => {
  const setup = makeWorkspace();
  for (const assignment of assignmentNames) {
    const groups = expressionGroups(setup.expressions, assignment);
    const ids = groups.flatMap((g) => g.expressions.map((e) => e.id));
    assert.equal(ids.length, new Set(ids).size);
    assert.deepEqual(ids.sort(), setup.expressions.map((e) => e.id).sort());
  }
  assert.equal(expressionGroups(setup.expressions, 'open')[0].name, 'Reveal');
  assert.equal(expressionGroups(setup.expressions, 'typing')[0].name, 'Flow');
});

test('playback clock resumes, loops, and emits each step once across frame boundaries', async () => {
  const { PlaybackClock, routinePlan } = await import('./playback.ts');
  const plan = routinePlan('typing-submit');
  const clock = new PlaybackClock(plan);
  clock.restart();
  const first = clock.advance(1200, true);
  assert.deepEqual(
    first.map((s) => s.action),
    ['typing', 'typing'],
  );
  // A pause leaves the clock untouched; the next advance starts at that position.
  assert.equal(clock.elapsed, 1200);
  const second = clock.advance(3000, true);
  assert.equal(second.filter((s) => s.action === 'submit').length, 1);
  const end = clock.advance(plan.duration - 4200, true);
  assert.equal(end.length, 0);
  assert.equal(clock.elapsed, 0);
  assert.deepEqual(clock.advance(1200, false), first);
  const rest = clock.advance(plan.duration, false);
  assert.equal(rest.filter((s) => s.action === 'submit').length, 1);
  assert.equal(clock.running, false);
  assert.deepEqual(clock.advance(10000, true), []);
});


test('Entrances alternates both opens with two-second holds and one-second gaps', async () => {
  const { PlaybackClock, routinePlan } = await import('./playback.ts');
  const clock = new PlaybackClock(routinePlan('entrances'));
  clock.restart();
  assert.deepEqual(clock.advance(0, true).map((s) => s.action), ['open']);
  assert.deepEqual(clock.advance(2000, true).map((s) => s.action), ['hide']);
  assert.deepEqual(clock.advance(1000, true).map((s) => s.action), ['openAfterIdle']);
  assert.deepEqual(clock.advance(2000, true).map((s) => s.action), ['hide']);
  assert.deepEqual(clock.advance(1000, true).map((s) => s.action), ['open']);
});
