'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Draggable graph probe. */
import { useEffect, useMemo, useRef, useState } from 'react';
import { Pause, Play, RotateCcw } from 'lucide-react';
import { ExpressionPreview } from './ExpressionPreview';
import { Button } from './ui/button';
import { dragDiagram } from './diagramDrag';
import { ExpressionPlayer } from '../../../vrsjmp/src/avatar/expression-player';
import type { AvatarSetup } from '@/lib/avatar/workspace';
export function ResponseLesson({
  setup,
  dark,
  momentum,
}: {
  setup: AvatarSetup;
  dark: boolean;
  momentum: boolean;
}) {
  const [restart, setRestart] = useState(0),
    [paused, setPaused] = useState(false),
    [delay, setDelay] = useState(0.3);
  const sampleSetup = useMemo(
    () => ({
      ...setup,
      assignments: {
        ...setup.assignments,
        working: null,
        typing:
          setup.expressions.find(
            (e) => e.id === (momentum ? 'nudge-momentum' : 'surface-react'),
          ) ?? null,
      },
    }),
    [setup, momentum],
  );
  const [player, setPlayer] = useState(() => new ExpressionPlayer(sampleSetup));
  const [clock, setClock] = useState(0);
  const [, wake] = useState(0);
  const [values, setValues] = useState<
    { time: number; speed: number; phase: number }[]
  >([]);
  const timers = useRef<ReturnType<typeof setTimeout>[]>([]);
  const sample = useRef({ player, time: -1 });
  const onSample = (p: ExpressionPlayer, paint: boolean) => {
    if (
      !paint ||
      (sample.current.player === p && p.time - sample.current.time < 1 / 20)
    )
      return;
    sample.current = { player: p, time: p.time };
    const signal = p.inspect('typing');
    setValues((previous) => [
      ...previous.filter((v) => v.time >= p.time - 5),
      { time: p.time, speed: signal.speed, phase: signal.phase },
    ]);
    setClock(p.time);
  };
  useEffect(
    () => () => {
      timers.current.forEach(clearTimeout);
      timers.current = [];
    },
    [player],
  );
  const left = Math.max(0, clock - 4),
    right = left + 4;
  const x = (time: number) => 20 + ((time - left) / 4) * 500;
  const y = (value: number) => 150 - Math.min(2.5, value) * 48;
  const series = (read: (time: number) => number) =>
    Array.from({ length: 241 }, (_, i) => {
      const time = left + i / 60;
      return `${i ? 'L' : 'M'}${x(time)},${y(read(time))}`;
    }).join(' ');
  const current = player.inspect('typing');
  const source = (time: number) => {
    const s = player.inspect('typing', time);
    return momentum ? s.input : s.ripple;
  };
  const at = clock - delay;
  const tap = () => {
    setPaused(false);
    player.trigger('typing');
    wake((n) => n + 1);
  };
  return (
    <div className="recipe-diagrams recipe-workbench response-lesson">
      <aside className="recipe-local-preview">
        <figure className="recipe-sphere">
          <figcaption>{momentum ? 'Momentum' : 'Ripple'}</figcaption>
          <div
            data-appearance={dark ? 'dark' : 'light'}
            className="response-sphere"
          >
            <ExpressionPreview
              key={restart}
              setup={sampleSetup}
              engine={player}
              onSample={onSample}
              dark={dark}
              zoom={3}
              live
              paused={paused}
            />
          </div>
          <small>96 px · 3×</small>
        </figure>
        <div className="response-actions">
          <Button onClick={tap}>Tap</Button>
          <Button
            onClick={() => {
              tap();
              timers.current.push(setTimeout(tap, 220));
            }}
          >
            Double tap
          </Button>
        </div>
      </aside>
      <div className="recipe-calculation">
        <p>
          {momentum
            ? 'A tap adds a short push. Repeated pushes build speed; resistance increases with speed, and friction slows the field between taps.'
            : 'Each tap adds a peak to the same signal. Points farther from the source read an earlier part of it, turning peaks in time into waves across the sphere.'}
        </p>
        <div className="response-actions">
          <Button
            onClick={() => {
              timers.current.forEach(clearTimeout);
              timers.current = [];
              setPaused(!paused);
            }}
          >
            {paused ? <Play size={13} /> : <Pause size={13} />}{' '}
            {paused ? 'Play' : 'Pause'}
          </Button>
          <Button
            onClick={() => {
              setRestart((n) => n + 1);
              setPlayer(new ExpressionPlayer(sampleSetup));
              setValues([]);
              setPaused(false);
            }}
          >
            <RotateCcw size={13} /> Reset
          </Button>
        </div>
        <svg
          viewBox="0 0 540 185"
          role="img"
          aria-label="Response curve: drag the probe"
          {...dragDiagram((px) =>
            setDelay(
              Math.max(0, Math.min(4, clock - (left + ((px - 20) / 500) * 4))),
            ),
          )}
        >
          <path d="M20 20V150H520" fill="none" stroke="var(--line)" />
          <path
            d={series(source)}
            fill="none"
            stroke="var(--lab-accent)"
            strokeWidth="2"
          />
          <path
            d={
              momentum
                ? values
                    .filter((p) => p.time >= left)
                    .map((p, i) => `${i ? 'L' : 'M'}${x(p.time)},${y(p.speed)}`)
                    .join(' ')
                : series((time) => player.inspect('typing', time).color)
            }
            fill="none"
            stroke="#789ba9"
            strokeWidth="2"
          />
          <path
            d={`M${x(at)} 20V150`}
            stroke="var(--muted)"
            strokeDasharray="3 4"
          />
          <circle
            cx={x(at)}
            cy={y(source(at))}
            r="4"
            fill="var(--lab-accent)"
          />
          <text x="20" y="175" fill="var(--muted)" fontSize="11">
            {left.toFixed(1)} s
          </text>
          <text x="490" y="175" fill="var(--muted)" fontSize="11">
            {right.toFixed(1)} s
          </text>
        </svg>
        <div className="response-legend">
          <span>● {momentum ? 'Push' : 'Wave source'}</span>
          <span>● {momentum ? 'Speed' : 'Color'}</span>
        </div>
        <div className="code-values">
          <div>
            <code>{momentum ? 'speed' : 'signal(t − delay)'}</code>
            <output>
              {(momentum ? current.speed : source(at)).toFixed(3)}
            </output>
          </div>
          <div>
            <code>{momentum ? 'phase' : 'delay'}</code>
            <output>
              {(momentum ? current.phase : delay).toFixed(3)}
              {momentum ? '' : ' s'}
            </output>
          </div>
        </div>
        <p>
          {momentum
            ? 'Speed is the rate of change of phase. The avatar continues from its current phase after every tap.'
            : 'Color follows its own response curve. It can fade gently while the wave remains sharp.'}
        </p>
      </div>
    </div>
  );
}
