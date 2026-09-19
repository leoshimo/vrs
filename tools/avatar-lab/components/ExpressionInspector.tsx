'use client';
import { RangeControl, SelectControl, Choices } from './AvatarControls';
import { paletteOptions } from '@/lib/avatar/avatar-palettes';
import { motionNames, type AvatarExpression } from '@/lib/avatar/workspace';
import {
  effectSettings,
  parameters,
  readPattern,
} from '../../../vrsjmp/src/avatar/effect-settings';
export function ExpressionInspector({
  expression,
  original,
  onChange,
  sustained = false,
}: {
  expression: AvatarExpression;
  original?: AvatarExpression;
  onChange: (e: AvatarExpression) => void;
  sustained?: boolean;
}) {
  function pattern(index: number, patch: Record<string, number>) {
    onChange({
      ...expression,
      patterns: expression.patterns.map((p, i) =>
        i === index ? readPattern(p.kind, { ...p.settings, ...patch }) : p,
      ),
    });
  }
  function resetField(key: 'revealStart' | 'distortionOnly') {
    if (!original) return;
    const next = { ...expression };
    if (key === 'revealStart') next.revealStart = original.revealStart;
    else next.distortionOnly = original.distortionOnly;
    if (!Object.hasOwn(original, key)) delete next[key];
    onChange(next);
  }
  function resetPattern(index: number, key: string) {
    const baseline = original?.patterns[index];
    if (!baseline || baseline.kind !== expression.patterns[index].kind) return;
    const settings: Record<string, number> = {
      ...expression.patterns[index].settings,
      [key]: (baseline.settings as Record<string, number>)[key],
    };
    if (!Object.hasOwn(baseline.settings, key)) delete settings[key];
    onChange({
      ...expression,
      patterns: expression.patterns.map((p, i) =>
        i === index ? readPattern(p.kind, settings) : p,
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
            defaultValue={original?.duration}
            min={0.1}
            max={2}
            step={0.01}
            format={(v) => `${v.toFixed(2)} s`}
            onChange={(duration) => onChange({ ...expression, duration })}
          />
        )}
      {!sustained &&
        expression.patterns.some(
          (p) => !['printed', 'fill', 'ripple'].includes(p.kind),
        ) && (
          <RangeControl
            label="Rise"
            value={
              expression.attack ??
              expression.duration * (expression.momentum ? 0.3 : 0.1)
            }
            defaultValue={
              original
                ? (original.attack ??
                  original.duration * (original.momentum ? 0.3 : 0.1))
                : undefined
            }
            min={0.01}
            max={expression.duration * 0.9}
            step={0.01}
            format={(v) => `${v.toFixed(2)} s`}
            onChange={(attack) => onChange({ ...expression, attack })}
          />
        )}
      {expression.patterns.some(
        (p) => p.kind === 'printed' || p.kind === 'fill',
      ) && (
        <RangeControl
          label="Initial coverage"
          value={expression.revealStart ?? 0}
          defaultValue={original ? (original.revealStart ?? 0) : undefined}
          onReset={() => resetField('revealStart')}
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
          {Object.entries(effectSettings[p.kind]).map(
            ([key, [label, min, max, step]]) =>
              key === 'direction' && p.kind === 'sweep' ? (
                <SelectControl
                  key={key}
                  label="Direction"
                  value={String(parameters(p).direction)}
                  options={[
                    ['0', 'Alternating'],
                    ['1', 'Left'],
                    ['2', 'Right'],
                    ['3', 'Up'],
                    ['4', 'Down'],
                  ]}
                  onChange={(v) => pattern(index, { direction: Number(v) })}
                />
              ) : (
                <RangeControl
                  key={key}
                  label={label}
                  value={parameters(p)[key]}
                  defaultValue={
                    original?.patterns[index]?.kind === p.kind
                      ? parameters(original.patterns[index])[key]
                      : undefined
                  }
                  onReset={() => resetPattern(index, key)}
                  min={min}
                  max={max}
                  step={step === 1 ? 1 : 0.01}
                  format={(v) => Number(v.toFixed(2)).toString()}
                  onChange={(value) => pattern(index, { [key]: value })}
                />
              ),
          )}
          {['swirl', 'orbit'].includes(p.kind) && (
            <Choices
              label="Shading"
              value={expression.distortionOnly ? 'flow' : 'crest'}
              defaultValue={
                original
                  ? original.distortionOnly
                    ? 'flow'
                    : 'crest'
                  : undefined
              }
              onReset={() => resetField('distortionOnly')}
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
          defaultValue={original?.color.mode}
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
                defaultValue={original?.color.palette}
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
              defaultValue={original?.color.strength}
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
              defaultValue={original?.color.attack}
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
              defaultValue={original?.color.duration}
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
