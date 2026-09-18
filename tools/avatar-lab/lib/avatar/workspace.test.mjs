import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const {
  makeWorkspace,
  readWorkspace,
  remix,
  removeExpression,
  assignmentNames,
  proposedAssignments,
} = await import('./workspace.ts');
const { makeLabSetup } = await import('./lab-setup.ts');
const { ExpressionPlayer } =
  await import('../../../../vrsjmp/src/avatar/expression-player.ts');
const step = (player, seconds) => {
  for (let i = 0; i < Math.round(seconds * 120); i++) player.frame(1 / 120);
};

test('simplified draft loads the proposal, removes copies and retains tuning', () => {
  const previous = makeWorkspace();
  previous.appearance.pitch = 2.6;
  previous.expressions.find(
    (e) => e.id === 'field-nudge',
  ).patterns[0].settings.typingEnergy = 0.91;
  previous.expressions.push({
    ...structuredClone(previous.expressions[0]),
    id: 'custom-remix',
    name: 'Lava copy — Remix',
    custom: true,
  });
  previous.assignments.open = 'gather';
  previous.assignments.submit = 'custom-remix';
  const next = readWorkspace(null, null, JSON.stringify(previous));
  assert.deepEqual(next.assignments, proposedAssignments);
  assert.equal(next.expressions.length, 22);
  assert.ok(
    next.expressions.every((e) => !e.custom && !/copy|remix/i.test(e.name)),
  );
  assert.equal(next.appearance.pitch, 2.6);
  assert.equal(
    next.expressions.find((e) => e.id === 'field-nudge').patterns[0].settings
      .typingEnergy,
    0.91,
  );
  // Loading the proposal is a migration, not a reset on each refresh.
  next.assignments.open = 'gather';
  next.assignments.typing = null;
  assert.deepEqual(
    readWorkspace(JSON.stringify(next)).assignments,
    next.assignments,
  );
});

test('migration preserves user tuning, variants and assignments', () => {
  const old = makeLabSetup();
  old.expressions.find(
    (e) => e.id === 'field-nudge',
  ).motions[0].settings.typingEnergy = 0.93;
  old.events.open = 'gather';
  old.events.typing = 'pressure-dimple';
  old.appearance.pitch = 2.3;
  const next = readWorkspace(null, JSON.stringify(old));
  assert.equal(next.assignments.open, 'gather');
  assert.equal(next.assignments.typing, 'pressure-dimple');
  assert.equal(next.appearance.pitch, 2.3);
  assert.equal(
    next.expressions.find((e) => e.id === 'field-nudge').patterns[0].settings
      .typingEnergy,
    0.93,
  );
  assert.deepEqual(readWorkspace(JSON.stringify(next)), next);
});
test('remixes are independent and deletion clears only their assignments', () => {
  const original = makeWorkspace();
  const next = remix(original, 'field-nudge', 'custom-test', 'typing');
  next.expressions.at(-1).patterns[0].settings.typingEnergy = 2;
  assert.notEqual(
    original.expressions.find((e) => e.id === 'field-nudge').patterns[0]
      .settings.typingEnergy,
    2,
  );
  assert.equal(next.assignments.typing, 'custom-test');
  const removed = removeExpression(next, 'custom-test');
  assert.equal(removed.assignments.typing, null);
  assert.equal(removed.assignments.working, original.assignments.working);
});
test('every expression can be driven by every assignment without non-finite output', () => {
  const setup = makeWorkspace();
  for (const expression of setup.expressions)
    for (const assignment of assignmentNames) {
      const player = new ExpressionPlayer({
        ...setup,
        assignments: { ...setup.assignments, [assignment]: expression.id },
      });
      player.trigger(assignment);
      const uniforms = player.frame(0.032);
      assert.ok(
        Object.values(uniforms).flat().every(Number.isFinite),
        `${expression.name}: ${assignment}`,
      );
    }
});
test('successive submissions add waves to the same source signal', () => {
  const setup = makeWorkspace();
  setup.assignments.submit = 'surface-react';
  setup.expressions.find(
    (e) => e.id === 'surface-react',
  ).patterns[0].settings.surfaceDuration = 1;
  const p = new ExpressionPlayer(setup);
  p.trigger('submit');
  step(p, 0.25);
  p.trigger('submit');
  assert.ok(p.inspect('submit', 0.03).ripple > 0);
  assert.equal(p.inspect('submit', 0.22).ripple, 0);
  step(p, 0.1);
  assert.ok(p.inspect('submit', 0.28).ripple > 0);
});
test('Sparks uses the same driver for Typing and gives successive bursts different angular signals', () => {
  const setup = makeWorkspace();
  setup.assignments.typing = 'sparks';
  const p = new ExpressionPlayer(setup);
  p.trigger('typing');
  step(p, 0.04);
  const first = p.frame(0)['u_randomHistory[0]'].slice(0, 4);
  step(p, 0.4);
  p.trigger('typing');
  step(p, 0.04);
  const second = p.frame(0)['u_randomHistory[0]'].slice(0, 4);
  assert.notDeepEqual(first, second);
  assert.ok(p.frame(0)['u_history[0]'][1] > 0);
});
test('motion and color timing are independent', () => {
  const setup = makeWorkspace(),
    e = setup.expressions.find((e) => e.id === 'field-nudge');
  e.duration = 0.15;
  e.color.duration = 0.7;
  const p = new ExpressionPlayer(setup);
  p.trigger('typing');
  step(p, 0.3);
  assert.equal(p.inspect('typing').input, 0);
  assert.ok(p.inspect('typing').color > 0);
});
test('response duration changes the emitted Sparks signal', () => {
  const setup = makeWorkspace();
  setup.assignments.typing = 'sparks';
  const expression = setup.expressions.find((e) => e.id === 'sparks');
  expression.duration = 0.1;
  const short = new ExpressionPlayer(structuredClone(setup));
  short.trigger('typing');
  step(short, 0.25);
  expression.duration = 0.5;
  const long = new ExpressionPlayer(setup);
  long.trigger('typing');
  step(long, 0.25);
  assert.equal(short.frame(0)['u_history[0]'][1], 0);
  assert.ok(long.frame(0)['u_history[0]'][1] > 0);
});
test('working release fades the same field without adding a new wave', () => {
  const p = new ExpressionPlayer(makeWorkspace());
  p.trigger('working');
  step(p, 0.5);
  const before = p.frame(0).u_mixAgitation[0];
  p.trigger('idle');
  const after = p.frame(0.008).u_mixAgitation[0];
  assert.ok(after > 0 && after < before);
  assert.equal(p.inspect('submit').input, 0);
  step(p, 1);
  assert.ok(p.frame(0).u_mixAgitation[0] < 0.001);
});

test('differently tuned ripples retain separate spatial channels while repeated taps share one curve', () => {
  const setup = makeWorkspace();
  setup.assignments.submit = 'surface-react';
  const p = new ExpressionPlayer(setup);
  p.trigger('submit');
  step(p, 0.1);
  p.trigger('submit');
  const first = p.frame(0);
  assert.equal(
    first['u_waveInfluences[0]']
      .filter((_, i) => i % 4 === 3)
      .reduce((a, b) => a + b, 0),
    1,
  );
  const oldOrigin = first['u_waveProfiles[0]'].slice(0, 3);
  setup.expressions.find(
    (e) => e.id === 'surface-react',
  ).patterns[0].settings.surfaceOriginX = 0.8;
  p.trigger('submit');
  step(p, 0.03);
  const next = p.frame(0);
  assert.deepEqual(next['u_waveProfiles[0]'].slice(0, 3), oldOrigin);
  assert.equal(next['u_waveProfiles[0]'][4], 0.8);
  assert.ok(next.waveTexture.slice(0, 128).some((v) => v > 0));
  assert.ok(next.waveTexture.slice(128, 256).some((v) => v > 0));
});


test('subtle Open migrates the former default once while retaining later choices', () => {
  const old = makeWorkspace();
  old.expressions = old.expressions.filter((e) => e.id !== 'printed-open');
  old.assignments.open = 'bloom';
  const next = readWorkspace(JSON.stringify(old));
  assert.equal(next.assignments.open, 'printed-open');
  next.assignments.open = 'bloom';
  assert.equal(readWorkspace(JSON.stringify(next)).assignments.open, 'bloom');
});
