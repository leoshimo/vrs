import type { Assignment, AvatarExpression } from './workspace';
import type { PlaybackAction } from '../../../../vrsjmp/src/avatar/expression-player';
import { completionExitMs } from '../../../../vrsjmp/src/palette-motion';
export type PlaybackStep = {
  at: number;
  action: PlaybackAction;
  query?: string;
};
export type PlaybackPlan = { duration: number; steps: PlaybackStep[] };
export type Routine = 'typing' | 'complete' | 'typing-submit' | 'entrances' | 'sequence';
export const routineNames: [Routine, string][] = [
  ['typing', 'Typing'],
  ['complete', 'Complete'],
  ['typing-submit', 'Typing → Complete'],
  ['entrances', 'Entrances'],
  ['sequence', 'Full'],
];
export function routinePlan(routine: Routine): PlaybackPlan {
  if (routine === 'entrances') return {
    duration: 6000,
    steps: [
      { at: 0, action: 'open', query: '' },
      { at: 2000, action: 'hide' },
      { at: 3000, action: 'openAfterIdle' },
      { at: 5000, action: 'hide' },
    ],
  };
  if (routine === 'sequence') return sequencePlan();
  if (routine === 'complete') return {
    duration: 5000,
    steps: [
      { at: 0, action: 'open', query: '' },
      { at: 700, action: 'submit' },
      { at: 800, action: 'working' },
      { at: 3200, action: 'complete' },
      { at: 3200 + completionExitMs, action: 'hide' },
    ],
  };
  const typing = {
    duration: routine === 'typing' ? 5000 : 7500,
    steps: [700, 1050, 1500, 1750, 2300].map((at, i) => ({
      at,
      action: 'typing' as const,
      query: 'Notes'.slice(0, i + 1),
    })),
  };
  return routine === 'typing'
    ? typing
    : {
        ...typing,
        steps: [
          { at: 0, action: 'open', query: '' },
          ...typing.steps,
          { at: 3900, action: 'submit' },
          { at: 4000, action: 'working' },
          { at: 5500, action: 'complete' },
          { at: 5500 + completionExitMs, action: 'hide' },
        ],
      };
}

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
      .map((p) => p.settings.surfaceDuration ?? 0.65) ?? []),
  );
}
export function sequencePlan(): PlaybackPlan {
  const text = 'Open notes';
  return {
    duration: 24000,
    steps: [
      { at: 0, action: 'hide', query: '' },
      { at: 1000, action: 'openAfterIdle' },
      ...[4000, 4350, 4750, 5050, 5600, 6250, 6600, 7000, 7350, 7700].map(
        (at, i) => ({
          at,
          action: 'typing' as const,
          query: text.slice(0, i + 1),
        }),
      ),
      { at: 9500, action: 'submit' },
      { at: 11500, action: 'working' },
      { at: 16500, action: 'complete' },
      { at: 16500 + completionExitMs, action: 'hide' },
      { at: 21000, action: 'open', query: '' },
      { at: 23500, action: 'hide' },
    ],
  };
}
export function repeatPlan(
  assignment: Assignment,
  expression?: AvatarExpression,
): PlaybackPlan {
  const settle = Math.max(2600, responseDuration(expression) * 1000 + 1200);
  if (assignment === 'open' || assignment === 'openAfterIdle')
    return {
      duration: settle + 800,
      steps: [
        { at: 0, action: 'hide' },
        { at: 500, action: assignment },
        { at: settle + 500, action: 'hide' },
      ],
    };
  if (assignment === 'typing')
    return {
      duration: 4000,
      steps: [0, 260, 620, 1000].map((at) => ({ at, action: assignment })),
    };
  if (assignment === 'complete')
    return {
      duration: settle + 500,
      steps: [
        { at: 0, action: assignment },
        { at: 280, action: assignment },
      ],
    };
  return { duration: 5000, steps: [{ at: 0, action: assignment }] };
}
