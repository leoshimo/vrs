import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { FrameClock } = await import('./frame-clock.ts');
const { SignalTimeline } = await import('./signal-timeline.ts');
const { neutralInk } = await import('./preview-colors.ts');
const { makeWorkspace } = await import('./workspace.ts');
const { comparisonSetup } = await import('./comparison.ts');
const { ExpressionPlayer } =
  await import('../../../../vrsjmp/src/avatar/expression-player.ts');

function scheduler() {
  let id = 0;
  const frames = new Map();
  const clock = new FrameClock(
    (callback) => {
      frames.set(++id, callback);
      return id;
    },
    (id) => frames.delete(id),
  );
  return {
    clock,
    frames,
    tick: (time) => {
      const batch = [...frames.values()];
      frames.clear();
      for (const callback of batch) callback(time);
    },
  };
}
test('frame jobs share a clock and leave no callbacks queued when paused or inactive', () => {
  const { clock, frames, tick } = scheduler();
  const deltas = [];
  const live = clock.task((delta) => {
    deltas.push(delta);
    return true;
  });
  const oneFrame = clock.task(() => false);
  assert.equal(frames.size, 0);
  live.wake();
  oneFrame.wake();
  assert.equal(frames.size, 1);
  tick(100);
  tick(116);
  assert.deepEqual(deltas, [0, 16]);
  live.sleep();
  assert.equal(frames.size, 0);
  live.wake();
  tick(5000);
  assert.equal(deltas.at(-1), 0);
  clock.setVisible(false);
  assert.equal(frames.size, 0);
  clock.setVisible(true);
  tick(9000);
  assert.equal(deltas.at(-1), 0);
  live.dispose(); // The sleeping one-frame job remains registered.
  assert.equal(frames.size, 0);
  oneFrame.dispose();
});
test('offscreen advancement matches rendered motion without building shader packets', () => {
  const setup = makeWorkspace();
  const a = new ExpressionPlayer(setup),
    b = new ExpressionPlayer(setup);
  for (let i = 0; i < 360; i++) {
    if (i === 10 || i === 18) {
      a.trigger('typing');
      b.trigger('typing');
    }
    if (i === 35) {
      a.trigger('working');
      b.trigger('working');
    }
    if (i === 220) {
      a.trigger('idle');
      b.trigger('idle');
    }
    a.advance(1 / 60);
    a.frame();
    b.advance(1 / 60);
  }
  assert.deepEqual(a.frame(), b.frame());
});
test('a triggered comparison can sleep after settling and wake for another input', () => {
  const setup = comparisonSetup(makeWorkspace(), {
    id: 'sparks',
    input: 'complete',
  });
  setup.assignments.idle = null;
  const player = new ExpressionPlayer(setup);
  assert.equal(player.needsAnimation(), false);
  player.trigger('complete');
  assert.equal(player.needsAnimation(), true);
  for (let i = 0; i < 360; i++) player.advance(1 / 60);
  assert.equal(player.needsAnimation(), false);
  player.trigger('complete');
  assert.equal(player.needsAnimation(), true);
  player.trigger('hide');
  assert.equal(player.needsAnimation(), false);
});
test('Working release retains long-travel waves until they leave the sphere', () => {
  const setup = comparisonSetup(makeWorkspace(), {
    id: 'working',
    input: 'working',
  });
  setup.assignments.idle = null;
  const expression = setup.expressions.find((e) => e.id === 'working');
  expression.patterns.find((p) => p.kind === 'ripple').settings.travel = 8;
  const player = new ExpressionPlayer(setup);
  player.trigger('working');
  for (let i = 0; i < 60; i++) player.advance(1 / 60);
  player.trigger('idle');
  for (let i = 0; i < 300; i++) player.advance(1 / 60);
  assert.equal(player.needsAnimation(), true);
  for (let i = 0; i < 300; i++) player.advance(1 / 60);
  assert.equal(player.needsAnimation(), false);
});
test('timeline keeps assignment signals separate, bounded, and paints only new samples', () => {
  const setup = makeWorkspace();
  const player = new ExpressionPlayer(setup),
    timeline = new SignalTimeline();
  let notifications = 0;
  const unsubscribe = timeline.subscribe(() => notifications++);
  for (let i = 0; i < 600; i++) {
    if (i === 480) player.trigger('typing');
    player.advance(1 / 60);
    timeline.record(player, false);
  }
  assert.equal(notifications, 1);
  assert.ok(timeline.samples.length <= 61);
  assert.ok(timeline.samples.at(-1).time - timeline.samples[0].time <= 4);
  assert.ok(
    timeline.samples.some((sample) => sample.signals.typing.motion > 0),
  );
  assert.ok(
    timeline.samples.every((sample) => sample.signals.idle.motion <= 1),
  );
  assert.ok(
    timeline.samples.every((sample) => sample.signals.idle.color === 0),
  );
  timeline.record(player, true);
  assert.equal(notifications, 2);
  timeline.record(player, true);
  assert.equal(notifications, 2);
  unsubscribe();
});
test('monochrome preview ink has no color cast in either appearance', () => {
  for (const dark of [false, true]) {
    const [r, g, b] = neutralInk(dark);
    assert.equal(r, g);
    assert.equal(g, b);
  }
  const setup = makeWorkspace();
  setup.appearance.color = 'mono';
  setup.appearance.activityColor = 'spectrum';
  const player = new ExpressionPlayer(setup);
  for (let i = 0; i < 60; i++) player.advance(1 / 60);
  assert.equal(player.signals().color, 0);
  player.trigger('working');
  for (let i = 0; i < 60; i++) player.advance(1 / 60);
  assert.ok(player.signals().color > 0);
  player.trigger('idle');
  for (let i = 0; i < 240; i++) player.advance(1 / 60);
  assert.ok(player.signals().color < 0.0001);
  assert.ok(player.frame()['u_accents[0]'].every((n) => n === 0));
});
