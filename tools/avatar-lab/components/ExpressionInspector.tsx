'use client';
import { RangeControl, SelectControl, Choices } from './AvatarControls';
import { paletteOptions } from '@/lib/avatar/avatar-palettes';
import {
  motionNames,
  type AvatarExpression,
  type Pattern,
} from '@/lib/avatar/workspace';
import type { SignalConfig } from '@/lib/avatar/signal-field';

type Knob = [keyof SignalConfig, string, number, number, number, number];
const knobs: Partial<Record<Pattern['kind'], Knob[]>> = {
  lava: [['idleAmount', 'Flow', 0, 2.4, 0.1, 1.6]],
  nudge: [['typingEnergy', 'Strength', 0.05, 1.5, 0.05, 0.6]],
  pressure: [
    ['typingEnergy', 'Strength', 0.05, 1.5, 0.05, 0.28],
    ['pressureSpread', 'Concentration', 0.5, 10, 0.1, 3],
    ['pressureOffset', 'Distance from center', 0, 0.95, 0.05, 0.36],
  ],
  tilt: [['typingEnergy', 'Strength', 0.05, 2, 0.05, 1]],
  ruffle: [['typingEnergy', 'Strength', 0.05, 2, 0.05, 0.5]],
  swirl: [
    ['idleSwirl', 'Strength', 0, 2, 0.05, 0.8],
    ['swirlRate', 'Speed', 0.1, 3, 0.1, 0.8],
  ],
  orbit: [
    ['workingOrbit', 'Strength', 0, 2, 0.05, 1],
    ['orbitRate', 'Speed', 0.1, 3, 0.1, 1.8],
  ],
  ripple: [
    ['surfaceAmplitude', 'Strength', 0, 1.5, 0.05, 0.45],
    ['surfaceDuration', 'Travel time', 0.15, 10, 0.05, 0.65],
    ['surfaceWidth', 'Width', 0.1, 0.8, 0.02, 0.32],
    ['surfaceOriginX', 'Origin X', -1, 1, 0.05, -0.7],
    ['surfaceOriginY', 'Origin Y', -1, 1, 0.05, -0.6],
  ],
  sparks: [
    ['dispatchEnergy', 'Strength', 0.1, 2, 0.1, 0.8],
    ['sparkCount', 'Count', 3, 18, 1, 11],
  ],
  bloom: [['dispatchEnergy', 'Strength', 0.1, 2, 0.1, 0.8]],
  gather: [['dispatchEnergy', 'Strength', 0.1, 2, 0.1, 1]],
  echo: [['dispatchEnergy', 'Strength', 0.1, 2, 0.1, 1]],
};
const movingKnobs: Knob[] = [
  ['loadingStrength', 'Strength', 0, 2.5, 0.05, 1],
  ['loadingRate', 'Speed', 0.1, 3, 0.05, 1],
  ['coupling', 'Distortion', 0.1, 3, 0.1, 1],
];
export function ExpressionInspector({
  expression,
  onChange,
  sustained = false,
}: {
  expression: AvatarExpression;
  onChange: (e: AvatarExpression) => void;
  sustained?: boolean;
}) {
  function pattern(index: number, patch: Partial<SignalConfig>) {
    onChange({
      ...expression,
      patterns: expression.patterns.map((p, i) =>
        i === index ? { ...p, settings: { ...p.settings, ...patch } } : p,
      ),
    });
  }
  return (
    <>
      {expression.patterns.some((p) => p.kind !== 'ripple') &&
        (!sustained ||
          expression.patterns.some(
            (p) => p.kind === 'printed' || p.kind === 'fill',
          )) && (
          <RangeControl
            label={
              expression.patterns.some(
                (p) => p.kind === 'printed' || p.kind === 'fill',
              )
                ? 'Entrance duration'
                : 'Duration'
            }
            value={expression.duration}
            min={0.1}
            max={2}
            step={0.01}
            format={(v) => `${v.toFixed(2)} s`}
            onChange={(duration) => onChange({ ...expression, duration })}
          />
        )}
      {expression.patterns.some((p) => p.kind === 'printed' || p.kind === 'fill') && (
        <RangeControl
          label="Initial coverage"
          value={expression.revealStart ?? 0}
          min={0}
          max={1}
          step={0.01}
          format={(v) => `${Math.round(v * 100)}%`}
          onChange={(revealStart) => onChange({ ...expression, revealStart })}
        />
      )}
      {expression.patterns.map((p, index) => (
        <section className="pattern-controls" key={`${p.kind}-${index}`}>
          {expression.patterns.length > 1 && <h3>{motionNames[p.kind]}</h3>}
          {(
            knobs[p.kind] ??
            (['stir', 'drift', 'eddies', 'sweep'].includes(p.kind)
              ? movingKnobs
              : [])
          ).map(([key, label, min, max, step, fallback]) => (
            <RangeControl
              key={key}
              label={label}
              value={Number(p.settings[key] ?? fallback)}
              min={min}
              max={max}
              step={step === 1 ? 1 : 0.01}
              format={(v) => Number(v.toFixed(2)).toString()}
              onChange={(value) => pattern(index, { [key]: value })}
            />
          ))}
          {['swirl', 'orbit'].includes(p.kind) && (
            <Choices
              label="Shading"
              value={expression.distortionOnly ? 'flow' : 'crest'}
              options={[
                ['flow', 'Field only'],
                ['crest', 'Field + crest'],
              ]}
              onChange={(v) =>
                onChange({ ...expression, distortionOnly: v === 'flow' })
              }
            />
          )}
        </section>
      ))}
      <section className="pattern-controls">
        <Choices
          label="Color"
          value={expression.color.mode}
          options={[
            ['none', 'Off'],
            ['accent', 'Avatar accent'],
            ['custom', 'Custom'],
          ]}
          onChange={(mode) =>
            onChange({ ...expression, color: { ...expression.color, mode } })
          }
        />
        {expression.color.mode !== 'none' && (
          <>
            {expression.color.mode === 'custom' && (
              <SelectControl
                label="Palette"
                value={expression.color.palette}
                options={paletteOptions}
                onChange={(palette) =>
                  onChange({
                    ...expression,
                    color: { ...expression.color, palette },
                  })
                }
              />
            )}
            <RangeControl
              label="Strength"
              value={expression.color.strength}
              min={0}
              max={0.8}
              step={0.05}
              format={(v) => `${Math.round(v * 100)}%`}
              onChange={(strength) =>
                onChange({
                  ...expression,
                  color: { ...expression.color, strength },
                })
              }
            />
            <RangeControl
              label="Rise"
              value={expression.color.attack}
              min={0.02}
              max={0.3}
              step={0.01}
              format={(v) => `${Math.round(v * 1000)} ms`}
              onChange={(attack) =>
                onChange({
                  ...expression,
                  color: { ...expression.color, attack },
                })
              }
            />
            <RangeControl
              label="Duration"
              value={expression.color.duration}
              min={0.1}
              max={1.5}
              step={0.01}
              format={(v) => `${v.toFixed(2)} s`}
              onChange={(duration) =>
                onChange({
                  ...expression,
                  color: { ...expression.color, duration },
                })
              }
            />
          </>
        )}
      </section>
    </>
  );
}
