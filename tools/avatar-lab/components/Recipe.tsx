'use client';
import { useEffect, useRef, useState } from 'react';
import { Play, Pause, ArrowLeft, ArrowRight } from 'lucide-react';
import { useMediaQuery } from './useMediaQuery';
import { useSetup } from './useSetup';
import { Button } from './ui/button';
import { Choices, RangeControl, SelectControl } from './AvatarControls';
import { PreviewAppearance, usePreviewAppearance } from './PreviewAppearance';
import { CompositionLesson } from './CompositionLesson';
import { ResponseLesson } from './ResponseLesson';
import { RecipeSphere } from './RecipeSphere';
import { ShaderFunctionDiagrams } from './ShaderFunctionDiagrams';
import { FlowLesson } from './FlowLesson';
import { CoordinateLab, coordinateNotes } from './CoordinateLab';
import { GradientLesson, PrintingLesson, ColorLesson } from './FieldLessons';
import {
  ClockLesson,
  BandLesson,
  CodeValues,
  HemisphereSection,
  ImpulseGraph,
  OperationValues,
  SurfaceWaveLesson,
  number,
} from './AnatomyParts';
import { clamp } from './diagramDrag';
import { assigned } from '@/lib/avatar/workspace';
import {
  impulseSample,
  sampleDisplacement,
  type CoordinateOperation,
} from '@/lib/avatar/field-explainer';
import type { SignalConfig } from '@/lib/avatar/signal-field';
const chapters = [
  [
    'Mask',
    'Coordinates are measured in radii. The center is (0, 0); a point on the edge has distance 1.',
  ],
  [
    'Soft edge',
    'A short fade around the boundary keeps the circle smooth between pixels.',
  ],
  [
    'Hemisphere',
    'The sphere’s radius is 1, so x² + y² + z² = 1. Once x and y are known, the remaining squared distance gives its height.',
  ],
  [
    'Gradient',
    'Project each position onto the light’s direction. Points toward the light gain coverage; points away from it lose coverage.',
  ],
  [
    'Flow',
    'A sine across x multiplies a cosine across y. Their moving peaks add local light and dark regions to Lava.',
  ],
  [
    'Lava',
    'Base coverage, hemisphere height, directional gradient, and flow combine before printing.',
  ],
  [
    'Printing',
    'A cell prints when coverage reaches its threshold. A repeating Bayer matrix distributes those thresholds.',
  ],
  ['Color', 'Position and phase select a pigment from the palette.'],
  [
    'Tone bands',
    'Group coverage into levels, then blend those levels into the coverage and pigment.',
  ],
  ['Coordinates', 'Move the coordinates used to sample the field.'],
  ['Clock', 'An advancing phase drives the gradient, flow, and palette.'],
  [
    'Tilt',
    'A keypress gives a damped spring a kick. The spring briefly shifts the light direction.',
  ],
  [
    'Ripple',
    'A ridge travels around the sphere, changing its outline, sampling coordinates, and shading.',
  ],
];
chapters.push(['Response curves', ''], ['Momentum', ''], ['Composition', '']);
const operations = {
  translation: 'primitiveDrift',
  twist: 'primitiveTwist',
  shear: 'primitiveNoise',
  vertical: 'gravity',
} as const;
function PointInput({
  axis,
  value,
  onChange,
}: {
  axis: string;
  value: number;
  onChange: (n: number) => void;
}) {
  const [draft, setDraft] = useState<string | null>(null);
  return (
    <label>
      {axis}
      <input
        type="number"
        aria-label={`Point ${axis}`}
        value={draft ?? Number(value.toFixed(3))}
        step="0.01"
        min="-1.1"
        max="1.1"
        onFocus={() => setDraft(String(Number(value.toFixed(3))))}
        onChange={(e) => {
          setDraft(e.target.value);
          const n = Number.parseFloat(e.target.value);
          if (Number.isFinite(n)) onChange(n);
        }}
        onBlur={() => setDraft(null)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') e.currentTarget.blur();
        }}
      />
    </label>
  );
}
export function Recipe() {
  const { setup } = useSetup();
  const [theme, setTheme] = usePreviewAppearance(),
    [step, setStep] = useState(0),
    [contentsOpen, setContentsOpen] = useState<boolean | null>(null);
  const wide = useMediaQuery('(min-width: 851px)'),
    reduce = useMediaQuery('(prefers-reduced-motion: reduce)');
  const [patch, setPatch] = useState<Partial<SignalConfig>>({}),
    [phase, setPhase] = useState(0),
    [playing, setPlaying] = useState(false);
  const [point, setPoint] = useState<[number, number]>([0.6, 0.25]),
    [amount, setAmount] = useState(0.8),
    [operation, setOperation] = useState<CoordinateOperation>('translation'),
    [wavePart, setWavePart] = useState('1');
  const [density, setDensity] = useState(0.5),
    [duration, setDuration] = useState(3);
  const chapter = useRef<HTMLHeadingElement>(null),
    playhead = useRef(phase);
  const [x, y] = point;
  const config: SignalConfig = {
    ...setup.appearance,
    ...Object.assign(
      {},
      ...(assigned(setup, 'idle')?.patterns.map((p) => p.settings) ?? []),
    ),
    ...patch,
    shape: 'pearl',
  };
  const volume = config.volume ?? 1,
    strength = config.lightStrength ?? 1,
    angle = config.lightAngle ?? 0,
    fill = config.fill ?? 0.5,
    detail = config.idleAmount ?? 1.9;
  const lightAngle =
    phase - 0.7 + 0.24 * Math.sin(phase * 1.7) + (angle * Math.PI) / 180;
  const dome = Math.sqrt(Math.max(0, 1 - x * x - y * y)),
    projection = x * Math.cos(lightAngle) + y * Math.sin(lightAngle);
  const flow =
    Math.sin(x * 2.8 + phase * 1.4) * Math.cos(y * 3.1 - phase) * 0.09 * detail;
  function densityAt(px: number, py: number) {
    const dome = Math.sqrt(Math.max(0, 1 - px * px - py * py));
    const proj = px * Math.cos(lightAngle) + py * Math.sin(lightAngle);
    const f =
      Math.sin(px * 2.8 + phase * 1.4) *
      Math.cos(py * 3.1 - phase) *
      0.09 *
      detail;
    const tone = clamp(
      (0.4 + 0.25 * dome * volume + 0.34 * proj * strength + f - 0.5) *
        (config.contrast ?? 1) +
        0.5,
      0.015,
      0.985,
    );
    return fill < 0.5 ? tone * fill * 2 : tone + (1 - tone) * (fill - 0.5) * 2;
  }
  function pickCoverage(target: number) {
    let best = Infinity,
      point: [number, number] = [x, y];
    for (let i = 0; i <= 40; i++)
      for (let j = 0; j <= 40; j++) {
        const px = i / 20 - 1,
          py = j / 20 - 1;
        if (px * px + py * py > 1) continue;
        const error =
          Math.abs(densityAt(px, py) - target) +
          Math.hypot(px - x, py - y) * 0.0001;
        if (error < best) {
          best = error;
          point = [px, py];
        }
      }
    setPoint(point);
  }
  const clockDisplacement = sampleDisplacement(
    x,
    y,
    phase,
    config.gravity ?? 0,
    'vertical',
  );
  const raw = 0.4 + 0.25 * dome * volume + 0.34 * projection * strength + flow;
  const contrasted = clamp(
    (raw - 0.5) * (config.contrast ?? 1) + 0.5,
    0.015,
    0.985,
  );
  const coverage =
    fill < 0.5
      ? contrasted * fill * 2
      : contrasted + (1 - contrasted) * (fill - 0.5) * 2;
  const impulse = impulseSample(phase, amount);
  const maxPhase = step === 12 ? 4 : step === 11 ? 1 : 8;
  function set(p: Partial<SignalConfig>) {
    setPatch((c) => ({ ...c, ...p }));
  }
  function pick(px: number, py: number) {
    let vx = clamp(px, -1.1, 1.1),
      vy = clamp(py, -1.1, 1.1);
    if (step === 2 || step === 12) {
      const r = Math.hypot(vx, vy);
      if (r > 1) {
        vx /= r;
        vy /= r;
      }
    }
    setPoint([vx, vy]);
  }
  function scrub(p: number) {
    setPlaying(false);
    setPhase(p);
  }
  function go(i: number) {
    setStep(i);
    if (i === 1) {
      const r = Math.hypot(x, y) || 1;
      setPoint([x / r, y / r]);
    }
    if (i === 2 && Math.hypot(x, y) > 0.95) setPoint([0.6, 0.25]);
    setPlaying(false);
    setPhase(i === 12 ? 1 : 0);
    if (i === 12) {
      const r = Math.hypot(x, y);
      if (r > 1) setPoint([x / r, y / r]);
    }
    requestAnimationFrame(() => {
      const heading = chapter.current;
      heading?.focus({ preventScroll: true });
      const bounds = heading?.getBoundingClientRect();
      const headerBottom =
        document.querySelector('.lab-header')?.getBoundingClientRect().bottom ??
        0;
      if (
        bounds &&
        (bounds.top < headerBottom || bounds.bottom > innerHeight)
      ) {
        heading
          ?.closest('.recipe-chapter')
          ?.scrollIntoView({ block: 'start', behavior: 'instant' });
      }
    });
  }
  useEffect(() => {
    playhead.current = phase;
  }, [phase]);
  useEffect(() => {
    if (!playing || reduce) return;
    let frame = 0,
      last = 0;
    const tick = (now: number) => {
      if (last) {
        const dt = Math.min(0.05, (now - last) / 1000);
        const next =
          playhead.current +
          dt * (step === 12 ? 4 / duration : step === 11 ? 1 : 0.38);
        playhead.current =
          next >= maxPhase ? (step === 11 ? 1 : next % maxPhase) : next;
        setPhase(playhead.current);
        if (step === 11 && next >= 1) {
          setPlaying(false);
          return;
        }
      }
      last = now;
      frame = requestAnimationFrame(tick);
    };
    const visibility = () => {
      cancelAnimationFrame(frame);
      last = 0;
      if (!document.hidden) frame = requestAnimationFrame(tick);
    };
    visibility();
    document.addEventListener('visibilitychange', visibility);
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener('visibilitychange', visibility);
    };
  }, [playing, reduce, step, maxPhase, duration]);
  const preview: SignalConfig = {
    ...config,
    breathe: false,
    gravity: step < 10 ? 0 : config.gravity,
    primitiveDrift: 0,
    primitiveTwist: 0,
    primitiveNoise: 0,
    inspectPhase: step === 11 ? 0 : phase,
    debugStage:
      step <= 1
        ? 1
        : step === 2
          ? 6
          : step === 3
            ? 7
            : step === 4
              ? 8
              : step === 5
                ? 9
                : 0,
    toneSteps: step < 8 ? 0 : config.toneSteps,
    ...(step <= 1 ? { fill: 1 } : {}),
    color:
      step < 7
        ? 'mono'
        : step === 7
          ? config.color === 'field'
            ? 'field'
            : 'opal'
          : config.color,
    ...(step === 7 ? { colorTiming: 'always', colorStrength: 1 } : {}),
    ...(step === 9 ? { [operations[operation]]: amount } : {}),
    ...(step === 11
      ? {
          inspectPoke: [impulse.value, 0] as [number, number],
          typingMotion: 'light',
        }
      : {}),
    ...(step === 12
      ? {
          surfaceWave: amount,
          surfacePhase: phase,
          surfaceWidth: 0.32,
          surfaceWavelength: Math.PI / 6,
          surfaceDamping: 0,
          surfaceOriginX: -0.3,
          surfaceOriginY: -0.45,
          surfaceSilhouette: 1,
          surfaceDisplacement: 1,
          surfaceShading: 1,
        }
      : {}),
  };
  const sphere = (label: string, overrides: Partial<SignalConfig> = {}) => (
    <RecipeSphere
      label={label}
      theme={theme}
      config={{ ...preview, ...overrides }}
      x={x}
      y={y}
      onPick={pick}
    />
  );
  const pointInputs = (
    <div className="recipe-point">
      <PointInput axis="x" value={x} onChange={(n) => pick(n, y)} />
      <PointInput axis="y" value={y} onChange={(n) => pick(x, n)} />
    </div>
  );
  const clock = (
    <div className="recipe-clock">
      <Button
        disabled={!!reduce}
        aria-label={playing ? 'Pause' : 'Play'}
        onClick={() => {
          if (step === 11 && phase >= 1) setPhase(0);
          setPlaying(!playing);
        }}
      >
        {playing ? <Pause size={14} /> : <Play size={14} />}
      </Button>
      <RangeControl
        label={step === 11 ? 'Time · seconds' : 'Phase'}
        value={phase}
        min={0}
        max={maxPhase}
        step={0.01}
        format={number}
        onChange={scrub}
      />
    </div>
  );
  return (
    <section className="recipe">
      <div className="recipe-layout">
        <details className="recipe-contents" open={contentsOpen ?? wide}>
          <summary
            onClick={(e) => {
              e.preventDefault();
              setContentsOpen(!(contentsOpen ?? wide));
            }}
          >
            Contents{' '}
            <span>
              {step + 1} / {chapters.length}
            </span>
          </summary>
          <nav aria-label="Recipe chapters">
            {chapters.map(([name], i) => (
              <button
                key={name}
                aria-current={step === i ? 'step' : undefined}
                onClick={() => go(i)}
              >
                <span>{String(i + 1).padStart(2, '0')}</span>
                {name}
              </button>
            ))}
          </nav>
        </details>
        <article className="recipe-chapter">
          <header className="recipe-chapter-header">
            <h1 ref={chapter} tabIndex={-1}>
              {chapters[step][0]}
            </h1>
            <PreviewAppearance theme={theme} onChange={setTheme} />
          </header>
          {step === 15 ? (
            <CompositionLesson setup={setup} dark={theme === 'dark'} />
          ) : step >= 13 ? (
            <ResponseLesson
              key={step}
              setup={setup}
              dark={theme === 'dark'}
              momentum={step === 14}
            />
          ) : (
            <div
              className="recipe-diagrams recipe-workbench"
              data-pair={step === 8}
            >
              <aside className="recipe-local-preview">
                {sphere(
                  step <= 1
                    ? 'Circle mask'
                    : step === 2
                      ? 'Hemisphere height'
                      : step === 3
                        ? 'Directional gradient'
                        : step === 4
                          ? 'Flow · zero at mid-gray'
                          : step === 5
                            ? 'Combined coverage'
                            : step === 8
                              ? 'Continuous'
                              : step === 11
                                ? 'Tilt'
                                : step === 12
                                  ? 'Ripple'
                                  : 'Lava',
                  step === 8 ? { toneSteps: 0 } : {},
                )}
                {step === 8 &&
                  sphere('Banded', {
                    toneSteps:
                      config.toneSteps && config.toneSteps > 1
                        ? config.toneSteps
                        : 4,
                  })}
                {pointInputs}
              </aside>
              <div className="recipe-calculation">
                <div className="recipe-intro">
                  <p className="recipe-deck">{chapters[step][1]}</p>
                  <div className="recipe-controls">
                    {step === 3 && (
                      <RangeControl
                        label="Light offset"
                        value={angle}
                        min={-180}
                        max={180}
                        step={1}
                        format={(n) => `${n}°`}
                        onChange={(lightAngle) => set({ lightAngle })}
                      />
                    )}
                    {step === 4 && (
                      <RangeControl
                        label="Flow amount"
                        value={detail}
                        min={0}
                        max={3}
                        step={0.05}
                        onChange={(idleAmount) => set({ idleAmount })}
                      />
                    )}
                    {step === 5 && (
                      <>
                        <RangeControl
                          label="Hemisphere weight"
                          value={volume}
                          min={0}
                          max={2}
                          step={0.05}
                          onChange={(volume) => set({ volume })}
                        />
                        <RangeControl
                          label="Gradient weight"
                          value={strength}
                          min={0}
                          max={2}
                          step={0.05}
                          onChange={(lightStrength) => set({ lightStrength })}
                        />
                        <RangeControl
                          label="Fill"
                          value={fill}
                          min={0}
                          max={1}
                          onChange={(fill) => set({ fill })}
                        />
                      </>
                    )}
                    {step === 6 && (
                      <>
                        <RangeControl
                          label="Patch coverage"
                          value={density}
                          min={0}
                          max={1}
                          onChange={setDensity}
                        />
                        <RangeControl
                          label="Cell size"
                          value={config.pitch}
                          min={0.6}
                          max={6}
                          step={0.1}
                          onChange={(pitch) => set({ pitch })}
                        />
                        <Choices
                          label="Matrix"
                          value={String(config.matrix ?? 8)}
                          options={[
                            ['2', '2 × 2'],
                            ['4', '4 × 4'],
                            ['8', '8 × 8'],
                          ]}
                          onChange={(v) =>
                            set({ matrix: Number(v) as 2 | 4 | 8 })
                          }
                        />
                      </>
                    )}
                    {step === 7 && (
                      <>
                        <Choices
                          label="Palette"
                          value={config.color === 'field' ? 'field' : 'opal'}
                          options={[
                            ['opal', 'Opal'],
                            ['field', 'Instrument'],
                          ]}
                          onChange={(color) => set({ color })}
                        />
                        <RangeControl
                          label="Saturation"
                          value={config.saturation ?? 0.45}
                          min={0}
                          max={1}
                          onChange={(saturation) => set({ saturation })}
                        />
                      </>
                    )}
                    {step === 8 && (
                      <RangeControl
                        label="Levels"
                        value={
                          config.toneSteps && config.toneSteps > 1
                            ? config.toneSteps
                            : 4
                        }
                        min={2}
                        max={8}
                        step={1}
                        onChange={(toneSteps) => set({ toneSteps })}
                      />
                    )}
                    {step === 9 && (
                      <SelectControl
                        label="Operation"
                        value={operation}
                        options={
                          Object.entries(coordinateNotes).map(
                            ([id, [name]]) => [id, name],
                          ) as [CoordinateOperation, string][]
                        }
                        onChange={setOperation}
                      />
                    )}
                    {(step === 9 || step >= 11) && (
                      <RangeControl
                        label="Amplitude"
                        value={amount}
                        min={0}
                        max={1.8}
                        step={0.05}
                        onChange={setAmount}
                      />
                    )}
                    {step === 12 && (
                      <>
                        <Choices
                          label="Calculation"
                          value={wavePart}
                          options={[
                            ['0', 'Distance'],
                            ['1', 'Ridge'],
                            ['2', 'Apply'],
                          ]}
                          onChange={setWavePart}
                        />
                        <RangeControl
                          label="Duration"
                          value={duration}
                          min={0.6}
                          max={8}
                          step={0.1}
                          onChange={setDuration}
                        />
                      </>
                    )}
                  </div>
                  {step >= 3 && clock}
                </div>
                {step <= 1 && (
                  <>
                    <ShaderFunctionDiagrams
                      step={0}
                      part={step === 0 ? 'length' : 'edge'}
                      x={x}
                      y={y}
                      angle={angle}
                      onPick={pick}
                    />
                    {step === 0 ? (
                      <>
                        <p>
                          Subtract 1 to measure the gap to the edge: negative
                          inside, zero on the edge, positive outside.
                        </p>
                        <CodeValues
                          rows={[
                            [
                              'distance = length(v) - 1.;',
                              number(Math.hypot(x, y) - 1),
                            ],
                          ]}
                        />
                      </>
                    ) : (
                      <p className="recipe-language-note">
                        <code>smoothstep</code>, <code>length</code>,{' '}
                        <code>dot</code>, and <code>sqrt</code> are GLSL
                        built-ins used by the shader.
                      </p>
                    )}
                  </>
                )}
                {step === 2 && (
                  <>
                    <HemisphereSection x={x} y={y} onPick={pick} />
                    <div className="recipe-math">
                      <span>r² = x² + y² = {number(x * x + y * y)}</span>
                      <span>z² = 1 − r² = {number(1 - x * x - y * y)}</span>
                      <span>z = √(1 − r²) = {number(dome)}</span>
                    </div>
                    <p>
                      <code>dot(v, v)</code> multiplies matching components and
                      adds them: x × x + y × y. That gives r². The square root
                      recovers z from z²; <code>max(0, …)</code> keeps points
                      beyond the disk from taking a square root of a negative
                      number.
                    </p>
                    <CodeValues
                      rows={[
                        ['r2 = dot(v, v);', number(x * x + y * y)],
                        ['dome = sqrt(max(0., 1. - r2));', number(dome)],
                      ]}
                    />
                  </>
                )}
                {step === 3 && (
                  <>
                    <GradientLesson
                      x={x}
                      y={y}
                      angle={(lightAngle * 180) / Math.PI}
                      strength={strength}
                      dark={theme === 'dark'}
                      onPick={pick}
                    />
                  </>
                )}
                {step === 4 && (
                  <FlowLesson
                    x={x}
                    y={y}
                    phase={phase}
                    detail={detail}
                    onPick={pick}
                  />
                )}
                {step === 5 && (
                  <>
                    <div className="recipe-terms">
                      <span>
                        Base <b>.400</b>
                      </span>
                      <span>
                        Hemisphere <b>{number(0.25 * dome * volume)}</b>
                      </span>
                      <span>
                        Gradient <b>{number(0.34 * projection * strength)}</b>
                      </span>
                      <span>
                        Flow <b>{number(flow)}</b>
                      </span>
                    </div>
                    <CodeValues
                      rows={[
                        [
                          'tone = .40 + .25 * dome * volume + .34 * projection * strength + flow;',
                          number(raw),
                        ],
                        [
                          'tone = clamp((tone - .5) * contrast + .5, .015, .985);',
                          number(contrasted),
                        ],
                        [
                          fill < 0.5
                            ? 'tone *= fill * 2.;'
                            : 'tone = mix(tone, 1., (fill - .5) * 2.);',
                          number(coverage),
                        ],
                      ]}
                    />
                    <p>
                      The flow term gives the broad hemisphere and gradient
                      extra peaks. Increasing fill moves coverage toward solid
                      ink.
                    </p>
                  </>
                )}
                {step === 6 && (
                  <PrintingLesson
                    density={density}
                    pitch={config.pitch}
                    matrix={config.matrix ?? 8}
                    dark={theme === 'dark'}
                  />
                )}
                {step === 7 && (
                  <ColorLesson
                    x={x}
                    y={y}
                    phase={phase}
                    density={coverage}
                    bands={0}
                    saturation={config.saturation ?? 0.45}
                    instrument={config.color === 'field'}
                    dark={theme === 'dark'}
                    showBands={false}
                    onPick={pick}
                  />
                )}
                {step === 8 && (
                  <BandLesson
                    onDensity={pickCoverage}
                    count={
                      config.toneSteps && config.toneSteps > 1
                        ? config.toneSteps
                        : 4
                    }
                    density={coverage}
                  />
                )}
                {step === 9 && (
                  <>
                    <CoordinateLab
                      sample={densityAt}
                      operation={operation}
                      amount={amount}
                      phase={phase}
                      x={x}
                      y={y}
                      onPick={pick}
                    />
                    <OperationValues
                      operation={operation}
                      amount={amount}
                      phase={phase}
                      x={x}
                      y={y}
                    />
                  </>
                )}
                {step === 10 && (
                  <ClockLesson
                    phase={phase}
                    onPhase={scrub}
                    x={x - clockDisplacement[0]}
                    y={y - clockDisplacement[1]}
                    lightOffset={(angle * Math.PI) / 180}
                    detail={detail}
                  />
                )}
                {step === 11 && (
                  <ImpulseGraph
                    time={phase}
                    amplitude={amount}
                    onTime={scrub}
                  />
                )}
                {step === 12 && (
                  <SurfaceWaveLesson
                    phase={phase}
                    amplitude={amount}
                    part={Number(wavePart)}
                    x={x}
                    y={y}
                    onPick={pick}
                    onPhase={scrub}
                  />
                )}
              </div>
            </div>
          )}
          <footer className="recipe-navigation">
            <Button disabled={step === 0} onClick={() => go(step - 1)}>
              <ArrowLeft size={14} />
              {step ? chapters[step - 1][0] : 'Previous'}
            </Button>
            <Button
              disabled={step === chapters.length - 1}
              onClick={() => go(step + 1)}
            >
              {step < chapters.length - 1 ? chapters[step + 1][0] : 'Next'}
              <ArrowRight size={14} />
            </Button>
          </footer>
        </article>
      </div>
    </section>
  );
}
