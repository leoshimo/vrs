import { parameters } from '../../../../vrsjmp/src/avatar/effect-settings';
import type { AvatarExpression } from './workspace';
import type { PlaybackAction } from '../../../../vrsjmp/src/avatar/expression-player';
export type PlaybackStep = {
  at: number;
  action: PlaybackAction;
  query?: string;
};
export type PlaybackPlan = { duration: number; steps: PlaybackStep[] };
export class PlaybackClock {
  elapsed = 0;
  next = 0;
  running = false;
  plan: PlaybackPlan;
  constructor(plan: PlaybackPlan) {
    this.plan = plan;
  }
  restart(plan = this.plan) {
    this.plan = plan;
    this.elapsed = 0;
    this.next = 0;
    this.running = true;
  }
  advance(delta: number, loop: boolean) {
    if (!this.running) return [];
    const events: PlaybackStep[] = [];
    this.elapsed += delta;
    while (this.running) {
      while (
        this.next < this.plan.steps.length &&
        this.plan.steps[this.next].at <= this.elapsed
      )
        events.push(this.plan.steps[this.next++]);
      if (this.elapsed < this.plan.duration) break;
      if (!loop) {
        this.running = false;
        break;
      }
      this.elapsed -= this.plan.duration;
      this.next = 0;
    }
    return events;
  }
}
export function responseDuration(expression?: AvatarExpression) {
  return Math.max(
    0.3,
    expression?.duration ?? 0,
    expression?.color.duration ?? 0,
    ...(expression?.patterns
      .filter((p) => p.kind === 'ripple')
      .map((p) => parameters(p).travel ?? 0.65) ?? []),
  );
}
