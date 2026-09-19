import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const {
  makeWorkspace,
  readWorkspace,
  configuration,
  assignmentNames,
  withConfigurationScope,
  candidateKey,
  makeCandidate,
  applyCandidate,
} = await import('./workspace.ts');
const { isAvatarSetup } =
  await import('../../../../vrsjmp/src/avatar/setup-validation.ts');
const { ExpressionPlayer } =
  await import('../../../../vrsjmp/src/avatar/expression-player.ts');
const step = (p, seconds) => {
  for (let i = 0; i < seconds * 120; i++) p.advance(1 / 120);
};
const assign = (setup, input, id) =>
  (setup.assignments[input] = structuredClone(
    setup.expressions.find((e) => e.id === id),
  ));
test('candidate edits, use, reset, and assignment revert have independent boundaries', () => {
  const workspace = makeWorkspace();
  const saved = configuration(workspace);
  const original = structuredClone(workspace.assignments.typing);
  const candidate = makeCandidate(workspace.assignments.typing);
  candidate.expression.duration = 0.8;
  candidate.expression.patterns[0].settings.strength = 0.5;
  assert.deepEqual(workspace.assignments.typing, original);
  assert.deepEqual(candidate.original, original);
  workspace.candidates[candidateKey('typing', original.id)] = candidate;
  const used = applyCandidate(workspace, 'typing', candidate);
  assert.equal(used.assignments.typing.duration, 0.8);
  assert.deepEqual(used.assignments.submit, saved.assignments.submit);
  candidate.expression.duration = 1.1;
  assert.equal(used.assignments.typing.duration, 0.8);
  const reset = makeCandidate(candidate.original);
  assert.deepEqual(reset.expression, original);
  assert.equal(used.assignments.typing.duration, 0.8);
  const reverted = withConfigurationScope(used, saved, 'typing');
  assert.deepEqual(reverted.assignments.typing, original);
  assert.equal(
    reverted.candidates[candidateKey('typing', original.id)].expression
      .duration,
    1.1,
  );
  assert.equal(
    applyCandidate(used, 'typing', makeCandidate(null)).assignments.typing,
    null,
  );
});
test('Save assignments excludes appearance and candidates, which survive a browser reload', () => {
  const workspace = makeWorkspace();
  const saved = configuration(workspace);
  const candidate = makeCandidate(workspace.assignments.typing);
  candidate.expression.duration = 1;
  workspace.candidates[candidateKey('typing', candidate.expression.id)] =
    candidate;
  workspace.assignments.typing.duration = 0.4;
  workspace.assignments.submit.duration = 0.6;
  workspace.appearance.pitch = 3;
  const next = withConfigurationScope(saved, workspace, 'assignments');
  assert.equal(next.assignments.typing.duration, 0.4);
  assert.equal(next.assignments.submit.duration, 0.6);
  assert.deepEqual(next.appearance, saved.appearance);
  assert.deepEqual(Object.keys(next), ['appearance', 'assignments']);
  assert.deepEqual(readWorkspace(JSON.stringify(workspace)), workspace);
});
test('saving and reverting one assignment leaves all other drafts alone', () => {
  const saved = configuration(makeWorkspace());
  const draft = makeWorkspace(saved);
  draft.assignments.typing.duration = 0.8;
  draft.assignments.submit.duration = 0.6;
  draft.appearance.pitch = 3;
  const updated = withConfigurationScope(saved, draft, 'submit');
  assert.equal(updated.assignments.submit.duration, 0.6);
  assert.deepEqual(updated.assignments.typing, saved.assignments.typing);
  assert.deepEqual(updated.appearance, saved.appearance);
  const reverted = withConfigurationScope(draft, saved, 'submit');
  assert.deepEqual(reverted.assignments.submit, saved.assignments.submit);
  assert.equal(reverted.assignments.typing.duration, 0.8);
  assert.equal(reverted.appearance.pitch, 3);
  assert.deepEqual(
    withConfigurationScope(saved, draft, 'appearance').assignments,
    saved.assignments,
  );
  updated.assignments.submit.duration = 1;
  assert.equal(draft.assignments.submit.duration, 0.6);
});

test('saving contains only appearance and independent assignment settings', () => {
  const setup = makeWorkspace();
  setup.assignments.typing.patterns[0].settings.strength = 0.7;
  assert.equal(setup.assignments.submit.patterns[0].settings.strength, 1.04);
  const saved = configuration(setup);
  assert.deepEqual(Object.keys(saved), ['appearance', 'assignments']);
  assert.equal(isAvatarSetup(saved), true);
  assert.deepEqual(configuration(readWorkspace(JSON.stringify(setup))), saved);
  saved.assignments.typing.duration = 0.6;
  assert.equal(setup.assignments.typing.duration, 0.24);
});
test('the previous draft retains appearance, tweaks and assignments without shared references', () => {
  const setup = makeWorkspace();
  const old = {
    version: 2,
    appearance: { ...setup.appearance, pitch: 2.1 },
    expressions: setup.expressions,
    assignments: Object.fromEntries(
      assignmentNames.map((a) => [a, setup.assignments[a]?.id ?? null]),
    ),
  };
  old.expressions.find((e) => e.id === 'field-nudge').duration = 0.5;
  const migrated = readWorkspace(JSON.stringify(old));
  assert.equal(migrated.appearance.pitch, 2.1);
  assert.equal(migrated.assignments.typing.duration, 0.5);
  assert.equal('version' in migrated, false);
  assert.deepEqual(readWorkspace(JSON.stringify(migrated)), migrated);
  assert.deepEqual(
    readWorkspace(JSON.stringify({ ...migrated, version: 2 })),
    migrated,
  );
  migrated.assignments.typing.duration = 0.7;
  assert.equal(migrated.assignments.submit.duration, 0.5);
});
test('invalid saved values cannot enter the renderer', () => {
  const setup = configuration(makeWorkspace());
  assert.equal(isAvatarSetup(setup), true);
  setup.assignments.idle.patterns[0].settings.flowSpeed = 'fast';
  assert.equal(isAvatarSetup(setup), false);
  assert.deepEqual(readWorkspace(JSON.stringify(setup)), makeWorkspace());
  assert.equal(isAvatarSetup({}), false);
});
test('appearance validation uses renderer defaults rather than the saved configuration', () => {
  const setup = configuration(makeWorkspace());
  delete setup.appearance.lightAngle;
  assert.equal(isAvatarSetup(setup), true);
  setup.appearance.lightAngle = 'left';
  assert.equal(isAvatarSetup(setup), false);
  delete setup.appearance.lightAngle;
  delete setup.appearance.pitch;
  assert.equal(isAvatarSetup(setup), false);
});
test('each catalog effect can be assigned to every input with finite renderer output', () => {
  const setup = makeWorkspace();
  for (const e of setup.expressions)
    for (const input of assignmentNames) {
      const candidate = structuredClone(setup);
      candidate.assignments[input] = e;
      const p = new ExpressionPlayer(candidate);
      p.trigger(input);
      step(p, 0.2);
      assert.ok(
        Object.values(p.frame()).flat().every(Number.isFinite),
        `${e.name} / ${input}`,
      );
    }
});
test('overlapping ripple taps add to one spatial signal and retain earlier waves', () => {
  const setup = makeWorkspace();
  assign(setup, 'submit', 'surface-react');
  setup.assignments.submit.patterns[0].settings.travel = 1;
  const p = new ExpressionPlayer(setup);
  p.trigger('submit');
  step(p, 0.25);
  p.trigger('submit');
  step(p, 0.1);
  assert.ok(p.inspect('submit', 0.03).ripple > 0);
  assert.equal(p.inspect('submit', 0.22).ripple, 0);
  assert.ok(p.inspect('submit', 0.28).ripple > 0);
  assert.equal(
    p
      .frame()
      ['u_waveInfluences[0]'].filter((_, i) => i % 4 === 3)
      .reduce((a, b) => a + b, 0),
    1,
  );
});
test('Submit keeps its response while Working ramps, then Gather overlaps the release', () => {
  const setup = makeWorkspace();
  assign(setup, 'submit', 'submit-gather');
  assign(setup, 'complete', 'complete-settle');
  const p = new ExpressionPlayer(setup);
  p.trigger('submit');
  p.trigger('working');
  step(p, 0.1);
  assert.ok(p.signals('submit').motion > 0.2);
  assert.ok(p.signals('working').motion > 0.2);
  step(p, 1);
  const before = p.signals('working').motion;
  p.trigger('complete');
  assert.equal(p.signals('working').motion, before);
  step(p, 1 / 120);
  assert.ok(p.signals('working').motion > before * 0.9);
  step(p, 0.1);
  assert.ok(p.signals('complete').motion > 0.2);
  assert.ok(p.signals('working').motion > 0.1);
  step(p, 2);
  assert.ok(p.signals('working').motion < 0.0001);
  assert.equal(p.visible, true);
});
test('working release adds no new ripple; color only comes from opted-in assignments', () => {
  const setup = makeWorkspace();
  setup.appearance.activityColor = 'spectrum';
  const p = new ExpressionPlayer(setup);
  step(p, 1);
  p.trigger('typing');
  step(p, 0.1);
  assert.equal(p.signals().color, 0);
  p.trigger('working');
  step(p, 0.5);
  assert.ok(p.signals().color > 0.1);
  p.trigger('idle');
  const source = p.inspect('working').ripple;
  step(p, 0.05);
  assert.ok(p.inspect('working').ripple <= source);
  step(p, 2);
  assert.ok(p.signals().color < 0.0001);
});
test('changing ripple origin leaves existing waves on their original spatial channel', () => {
  const setup = makeWorkspace();
  assign(setup, 'submit', 'surface-react');
  const p = new ExpressionPlayer(setup);
  p.trigger('submit');
  step(p, 0.1);
  const origin = p.frame()['u_waveProfiles[0]'].slice(0, 3);
  setup.assignments.submit.patterns[0].settings.originX = 0.8;
  p.trigger('submit');
  step(p, 0.03);
  const frame = p.frame();
  assert.deepEqual(frame['u_waveProfiles[0]'].slice(0, 3), origin);
  assert.equal(frame['u_waveProfiles[0]'][4], 0.8);
});
