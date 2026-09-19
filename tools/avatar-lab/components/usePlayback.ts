'use client';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  assigned,
  assignmentLabels,
  type Assignment,
  type AvatarSetup,
} from '@/lib/avatar/workspace';
import { PlaybackClock, responseDuration } from '@/lib/avatar/playback';
import {
  preset,
  sequencePlan,
  cellSeconds,
  type Sequence,
  type SequenceName,
} from '@/lib/avatar/sequence';
import { frameTask } from '@/lib/avatar/frame-clock';
import type { PlaybackAction } from '../../../vrsjmp/src/avatar/expression-player';
import type { PreviewCommand } from './ExpressionPreview';

export function usePlayback(setup: AvatarSetup) {
  const current = useRef(setup);
  useEffect(() => {
    current.current = setup;
  }, [setup]);
  const [command, setCommand] = useState<PreviewCommand>({
    serial: 0,
    events: [],
  });
  const [shown, setShown] = useState(true),
    [working, setWorking] = useState(false);
  const [paused, setPaused] = useState(false),
    [running, setRunning] = useState(false);
  const [name, setName] = useState<SequenceName>('action');
  const [manual, setManual] = useState(true);
  const [sequences, setSequences] = useState<
    Partial<Record<SequenceName, Sequence>>
  >({});
  const sequence = sequences[name] ?? preset(name);
  const [query, setQuery] = useState(''),
    [pulses, setPulses] = useState<Assignment[]>([]);
  const clock = useRef(new PlaybackClock(sequencePlan(sequence)));
  const pauseRef = useRef(false),
    expiry = useRef(new Map<Assignment, number>());
  const driver = useRef<ReturnType<typeof frameTask> | null>(null);
  const progress = useRef<HTMLDivElement>(null);
  const send = useCallback(
    (action: PlaybackAction | 'reset' | 'resetHidden') => {
      setCommand((c) => ({
        serial: c.serial + 1,
        events: [...c.events, { serial: c.serial + 1, action }].slice(-128),
      }));
      if (action === 'reset' || action === 'resetHidden') {
        setShown(action === 'reset');
        setWorking(false);
        expiry.current.clear();
        setPulses([]);
      } else if (action === 'hide') {
        setShown(false);
        setWorking(false);
      } else if (action === 'idle' || action === 'working')
        setWorking(action === 'working');
      else {
        setShown(true);
        if (action === 'complete') setWorking(false);
        expiry.current.set(
          action,
          responseDuration(assigned(current.current, action)) * 1000,
        );
        setPulses([...expiry.current.keys()]);
      }
      driver.current?.wake();
    },
    [],
  );
  useEffect(() => {
    const task = frameTask((delta) => {
      if (pauseRef.current) return false;
      let expired = false;
      for (const [a, ms] of expiry.current) {
        if (ms <= delta) {
          expiry.current.delete(a);
          expired = true;
        } else expiry.current.set(a, ms - delta);
      }
      if (expired) setPulses([...expiry.current.keys()]);
      for (const step of clock.current.advance(delta, true)) {
        if (step.query !== undefined) setQuery(step.query);
        send(step.action);
      }
      progress.current?.style.setProperty(
        '--position',
        `${(clock.current.elapsed / clock.current.plan.duration) * 100}%`,
      );
      return clock.current.running || expiry.current.size > 0;
    });
    driver.current = task;
    return () => {
      task.dispose();
      driver.current = null;
    };
  }, [send]);
  function unpause() {
    pauseRef.current = false;
    setPaused(false);
    driver.current?.wake();
  }
  function start(next: Sequence) {
    send('resetHidden');
    setQuery('');
    clock.current.restart(sequencePlan(next));
    setRunning(true);
    unpause();
  }
  function chooseSequence(next: SequenceName | 'manual') {
    setManual(next === 'manual');
    if (next === 'manual') {
      clock.current.running = false;
      setRunning(false);
      unpause();
      return;
    }
    setName(next);
    start(sequences[next] ?? preset(next));
  }
  function editSequence(next: Sequence) {
    setSequences((s) => ({ ...s, [name]: next }));
    {
      clock.current.plan = sequencePlan(next);
      clock.current.elapsed = Math.min(
        clock.current.elapsed,
        clock.current.plan.duration - 1,
      );
      clock.current.next = clock.current.plan.steps.findIndex(
        (s) => s.at > clock.current.elapsed,
      );
      if (clock.current.next < 0)
        clock.current.next = clock.current.plan.steps.length;
      if (clock.current.running) {
        const cell = Math.floor(clock.current.elapsed / (cellSeconds * 1000));
        if (next.cells.window.includes(cell) !== shown)
          send(next.cells.window.includes(cell) ? 'open' : 'hide');
        if (next.cells.working.includes(cell) !== working)
          send(next.cells.working.includes(cell) ? 'working' : 'idle');
      }
    }
  }
  const active: Assignment[] = shown
    ? ['idle', ...(working ? ['working' as const] : []), ...pulses]
    : [];
  return {
    command,
    shown,
    working,
    paused,
    running,
    name,
    manual,
    sequence,
    progressRef: progress,
    query,
    setQuery,
    active,
    edited: JSON.stringify(sequence) !== JSON.stringify(preset(name)),
    status: paused
      ? 'Paused'
      : shown
        ? active
            .map(
              (a) =>
                `${assignmentLabels[a]}${assigned(setup, a) ? ` · ${assigned(setup, a)!.name}` : ''}`,
            )
            .join(' / ')
        : 'Hidden',
    chooseSequence,
    editSequence,
    resetSequence: () => editSequence(preset(name)),
    act: (action: PlaybackAction) => {
      setManual(true);
      clock.current.running = false;
      setRunning(false);
      unpause();
      send(action);
    },
    togglePlay: () => {
      if (running) {
        clock.current.running = false;
        setRunning(false);
        pauseRef.current = true;
        setPaused(true);
        driver.current?.sleep();
      } else {
        // Resume the chosen sequence's held inputs after a manual gesture.
        const cell = Math.floor(clock.current.elapsed / (cellSeconds * 1000));
        if (sequence.cells.window.includes(cell) !== shown)
          send(sequence.cells.window.includes(cell) ? 'open' : 'hide');
        if (sequence.cells.working.includes(cell) !== working)
          send(sequence.cells.working.includes(cell) ? 'working' : 'idle');
        clock.current.running = true;
        setRunning(true);
        unpause();
      }
    },
  };
}
