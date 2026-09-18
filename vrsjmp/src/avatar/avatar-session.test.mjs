import { test } from "node:test";
import assert from "node:assert/strict";
import { register } from "node:module";
register("./test-resolve.mjs", import.meta.url);
const { AvatarSession } = await import("./avatar-session.ts");
const { advancePreview } = await import("./preview-frame.ts");
const { ExpressionPlayer } = await import("./expression-player.ts");
const { assignedSetup } = await import("./assigned-setup.ts");
const advance = (player, seconds) => {
  for (let i = 0; i < seconds * 120; i++) player.advance(1 / 120);
};

test("native entrances use time hidden and duplicate opens do not retrigger", () => {
  const session = new AvatarSession();
  session.open(0);
  advance(session.player, 0.08);
  assert.ok(session.player.signals("openAfterIdle").motion > 0);
  const motion = session.player.signals("openAfterIdle").motion;
  session.open(10);
  assert.equal(session.player.signals("openAfterIdle").motion, motion);
  advance(session.player, 1);
  session.hide(90_000);
  session.open(90_100);
  advance(session.player, 0.08);
  assert.ok(session.player.signals("open").motion > 0);
  assert.equal(session.player.signals("openAfterIdle").motion, 0);
  session.hide(91_000);
  advance(session.player, 1);
  session.open(121_001);
  advance(session.player, 0.08);
  assert.ok(session.player.signals("openAfterIdle").motion > 0);
});
test("native input retains Working and only Working supplies accent", () => {
  const session = new AvatarSession();
  session.player.setup.appearance.activityColor = "spectrum";
  session.open(0);
  advance(session.player, 1);
  session.type();
  session.submit();
  advance(session.player, 0.1);
  assert.equal(session.player.signals().color, 0);
  session.working(true);
  advance(session.player, 0.5);
  session.type();
  session.submit();
  advance(session.player, 0.1);
  assert.equal(session.player.working, true);
  assert.ok(session.player.signals().color > 0);
  session.working(false);
  advance(session.player, 2);
  assert.ok(session.player.signals().color < 0.0001);
  session.hide(100);
  const time = session.player.time;
  session.type();
  session.submit();
  advance(session.player, 1);
  assert.equal(session.player.time, time);
  assert.equal(session.player.visible, false);
});
test("reduced motion completes a printed entrance without leaving an invisible avatar", () => {
  const session = new AvatarSession();
  session.reduced = true;
  session.open(0);
  assert.equal(session.player.frame(0).u_appearance, 1);
  assert.equal(session.player.frame(0).u_enableOrb, 1);
});
test("flat input history keeps advancing without redrawing the inactive avatar", () => {
  const setup = assignedSetup();
  for (const key of Object.keys(setup.assignments))
    setup.assignments[key] = key === "typing" ? "rest" : null;
  const player = new ExpressionPlayer(setup);
  const state = { live: true, paused: false, visible: true, dirty: false, trackTime: true };
  for (let i = 0; i < 120; i++) {
    const frame = advancePreview(player, state, 1 / 120);
    assert.equal(frame.draw, false);
    assert.equal(frame.keepAlive, true);
  }
  assert.ok(player.time > 0.99);
  assert.equal(player.phase, 0);
  player.trigger("typing");
  for (let i = 0; i < 600; i++) advancePreview(player, state, 1 / 120);
  assert.ok(player.phase > 0);
  const phase = player.phase;
  for (let i = 0; i < 120; i++) assert.equal(advancePreview(player, state, 1 / 120).draw, false);
  assert.equal(player.phase, phase);
  const time = player.time;
  const paused = advancePreview(player, { ...state, paused: true }, 1);
  assert.equal(paused.keepAlive, false);
  assert.equal(player.time, time);
});


test("normal Open briefly fills the print without a ripple or resetting the field", () => {
  const player = new ExpressionPlayer(assignedSetup());
  advance(player, 1);
  const phase = player.phase;
  player.trigger("hide");
  player.trigger("open");
  assert.equal(player.phase, phase);
  assert.equal(player.frame(0).u_appearance, 0.72);
  advance(player, 0.075);
  assert.ok(player.frame(0).u_appearance > 0.8);
  assert.equal(player.inspect("open").ripple, 0);
  advance(player, 0.08);
  assert.equal(player.frame(0).u_appearance, 1);
  player.trigger("openAfterIdle");
  assert.equal(player.frame(0).u_appearance, 0);
});
