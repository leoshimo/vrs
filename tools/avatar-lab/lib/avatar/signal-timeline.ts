import { assignmentNames, type Assignment } from './workspace';
import type { ExpressionPlayer } from '../../../../vrsjmp/src/avatar/expression-player';
export type SignalSample = {
  time: number;
  signals: Record<Assignment, { motion: number; color: number }>;
};
export class SignalTimeline {
  samples: SignalSample[] = [];
  private player?: ExpressionPlayer;
  private painted = -1;
  private listeners = new Set<() => void>();
  record(player: ExpressionPlayer, paint: boolean) {
    if (
      this.player !== player ||
      player.time < (this.samples.at(-1)?.time ?? 0)
    ) {
      this.player = player;
      this.samples = [];
      this.painted = -1;
    }
    if (
      !this.samples.length ||
      player.time - this.samples.at(-1)!.time >= 1 / 15
    ) {
      this.samples.push({
        time: player.time,
        signals: assignmentNames.reduce(
          (signals, a) => {
            signals[a] = player.signals(a);
            return signals;
          },
          {} as SignalSample['signals'],
        ),
      });
      while (this.samples.length > 1 && this.samples[0].time < player.time - 4)
        this.samples.shift();
    }
    const time = this.samples.at(-1)?.time ?? -1;
    if (paint && this.painted !== time) {
      this.painted = time;
      this.listeners.forEach((listener) => listener());
    }
  }
  subscribe(listener: () => void) {
    this.listeners.add(listener);
    listener();
    return () => {
      this.listeners.delete(listener);
    };
  }
}
