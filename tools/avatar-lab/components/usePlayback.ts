'use client';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  assigned,
  assignmentLabels,
  type Assignment,
  type AvatarSetup,
} from '@/lib/avatar/workspace';
import {
  PlaybackClock,
  responseDuration,
  routinePlan,
  type Routine,
} from '@/lib/avatar/playback';
import { frameTask } from '@/lib/avatar/frame-clock';
import type { PlaybackAction } from '../../../vrsjmp/src/avatar/expression-player';
import type { PreviewCommand } from './ExpressionPreview';
export type PlaybackMode = Routine | 'manual';

export function usePlayback(setup: AvatarSetup) {
  const current = useRef(setup);
  useEffect(() => {
    current.current = setup;
  }, [setup]);
  const [command, setCommand] = useState<PreviewCommand>({
    serial: 0,
    action: 'idle',
  });
  const [shown, setShown] = useState(true),
    [working, setWorking] = useState(false);
  const [paused, setPaused] = useState(false),
    [mode, setMode] = useState<PlaybackMode>('manual');
  const [query, setQuery] = useState(''),
    [transient, setTransient] = useState<Assignment | null>(null);
  const clock = useRef(new PlaybackClock(routinePlan('sequence')));
  const pauseRef = useRef(false),
    remaining = useRef(0);
  const driver = useRef<ReturnType<typeof frameTask> | null>(null);
  const send = useCallback(
    (action: PlaybackAction | 'reset' | 'resetHidden') => {
      setCommand((c) => ({ serial: c.serial + 1, action }));
      setTransient(null);
      remaining.current = 0;
      if (action === 'reset' || action === 'resetHidden') {
        setShown(action === 'reset');
        setWorking(false);
      } else if (action === 'hide') setShown(false);
      else {
        setShown(true);
        if (action === 'idle' || action === 'working')
          setWorking(action === 'working');
        else {
          setTransient(action);
          remaining.current =
            responseDuration(assigned(current.current, action)) * 1000;
        }
      }
      driver.current?.wake();
    },
    [],
  );
  useEffect(() => {
    const task = frameTask((delta) => {
      if (pauseRef.current) return false;
      if (remaining.current > 0) {
        remaining.current -= delta;
        if (remaining.current <= 0) setTransient(null);
      }
      for (const step of clock.current.advance(delta, true)) {
        if (step.query !== undefined) setQuery(step.query);
        send(step.action);
      }
      return clock.current.running || remaining.current > 0;
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
  const active: Assignment[] =
    shown && !paused
      ? [
          ...(working ? ['working' as const] : ['idle' as const]),
          ...(transient ? [transient] : []),
        ]
      : [];
  return {
    command,
    shown,
    working,
    paused,
    query,
    setQuery,
    active,
    mode,
    status: paused
      ? 'Paused'
      : shown
        ? active
            .map(
              (a) =>
                `${assignmentLabels[a]}${assigned(setup, a) ? ` · ${assigned(setup, a)!.name}` : ''}`,
            )
            .join(' / ')
        : '',
    act: (action: PlaybackAction) => {
      clock.current.running = false;
      setMode('manual');
      unpause();
      send(action);
    },
    chooseMode: (next: PlaybackMode) => {
      setMode(next);
      unpause();
      if (next === 'manual') clock.current.running = false;
      else {
        clock.current.restart(routinePlan(next));
        setQuery('');
        send(next === 'sequence' || next === 'entrances' ? 'resetHidden' : 'reset');
      }
    },
    togglePlay: () => {
      pauseRef.current = !pauseRef.current;
      setPaused(pauseRef.current);
      if (pauseRef.current) driver.current?.sleep();
      else driver.current?.wake();
    },
  };
}
