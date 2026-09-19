import type { PlaybackPlan, PlaybackStep } from './playback';
export const rails = [
  'window',
  'typing',
  'submit',
  'working',
  'complete',
] as const;
export type Rail = (typeof rails)[number];
export type Sequence = {
  duration: number;
  cells: Record<Rail, number[]>;
  entrances: Record<number, 'open' | 'openAfterIdle'>;
};
export const sequenceNames = {
  action: 'Full loop',
  quick: 'Quick action',
  repeated: 'Repeated submit',
  page: 'Load a page',
  entrances: 'Open & close',
  typing: 'Typing',
  idle: 'Idle',
};
export type SequenceName = keyof typeof sequenceNames;
export const cellSeconds = 0.25;
export const cellAt = (seconds: number) => Math.round(seconds / cellSeconds);
const interval = (start: number, end: number) =>
  Array.from({ length: cellAt(end - start) }, (_, i) => cellAt(start) + i);
export function preset(name: SequenceName): Sequence {
  const sequence: Sequence = {
    duration: 12,
    cells: {
      window: interval(0.5, 11),
      typing: interval(2, 4.5),
      submit: [cellAt(5)],
      working: interval(5, 9),
      complete: [cellAt(9)],
    },
    entrances: { [cellAt(0.5)]: 'openAfterIdle' },
  };
  if (name === 'page') sequence.cells.window = interval(0.5, 12);
  if (name === 'quick') {
    sequence.duration = 7;
    sequence.cells = {
      window: interval(0.5, 4),
      typing: interval(1.5, 3),
      submit: [cellAt(4)],
      working: [],
      complete: [],
    };
  }
  if (name === 'repeated') {
    sequence.cells.submit = [4.5, 4.75, 5].map(cellAt);
  }
  if (name === 'entrances') {
    sequence.duration = 6;
    sequence.cells = {
      window: interval(0.5, 4.5),
      typing: [],
      submit: [],
      working: [],
      complete: [],
    };
    sequence.entrances = {};
  }
  if (name === 'typing' || name === 'idle') {
    sequence.duration = 8;
    sequence.cells = {
      window: interval(0, 8),
      typing: name === 'typing' ? [...interval(1, 3), ...interval(4.5, 6)] : [],
      submit: [],
      working: [],
      complete: [],
    };
    sequence.entrances = {};
  }
  return sequence;
}
export function sequencePlan(sequence: Sequence): PlaybackPlan {
  const steps: PlaybackStep[] = [
    { at: 0, action: 'hide', query: '' },
    { at: 0, action: 'idle' },
  ];
  const cells = (rail: Rail) =>
    new Set(
      sequence.cells[rail].filter(
        (n) => n >= 0 && n * cellSeconds < sequence.duration,
      ),
    );
  const windows = cells('window');
  const work = cells('working');
  const typing = cells('typing');
  const submit = cells('submit');
  const complete = cells('complete');
  let letters = 0;
  for (let n = 0; n < cellAt(sequence.duration); n++) {
    const at = n * cellSeconds * 1000;
    if (windows.has(n) && !windows.has(n - 1))
      steps.push({ at, action: sequence.entrances[n] ?? 'open', query: '' });
    const closing = !windows.has(n) && windows.has(n - 1);
    if (!work.has(n) && work.has(n - 1)) steps.push({ at, action: 'idle' });
    if (!windows.has(n) && !closing) continue;
    if (submit.has(n)) steps.push({ at, action: 'submit' });
    if (work.has(n) && (!work.has(n - 1) || !windows.has(n - 1)))
      steps.push({ at, action: 'working' });
    if (complete.has(n)) steps.push({ at, action: 'complete' });
    if (closing) steps.push({ at, action: 'hide' });
    if (windows.has(n) && typing.has(n)) {
      steps.push({
        at,
        action: 'typing',
        query: 'Open notes'.slice(0, ++letters % 10 || 10),
      });
      if (n % 4 === 0) steps.push({ at: at + 130, action: 'typing' });
    }
  }
  return {
    duration: sequence.duration * 1000,
    steps: steps.sort((a, b) => a.at - b.at),
  };
}
export function paintCell(
  sequence: Sequence,
  rail: Rail,
  cell: number,
  on: boolean,
): Sequence {
  const cells = new Set(sequence.cells[rail]);
  if (on) cells.add(cell);
  else cells.delete(cell);
  return {
    ...sequence,
    cells: { ...sequence.cells, [rail]: [...cells].sort((a, b) => a - b) },
  };
}
