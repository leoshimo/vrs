'use client';
import { useId } from 'react';
import { Button } from '@/components/ui/button';
import { paletteOptions } from '@/lib/avatar/avatar-palettes';
import type { SignalConfig } from '@/lib/avatar/signal-field';
function ResetValue<T>({
  label,
  value,
  defaultValue,
  onReset,
}: {
  label: string;
  value: T;
  defaultValue?: T;
  onReset: () => void;
}) {
  if (defaultValue === undefined || Object.is(value, defaultValue)) return null;
  return (
    <button
      type="button"
      className="control-reset"
      aria-label={`Reset ${label}`}
      onClick={onReset}
    >
      Reset
    </button>
  );
}
export function RangeControl({
  label,
  value,
  min,
  max,
  step = 0.01,
  onChange,
  format,
  description,
  defaultValue,
  onReset,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  format?: (value: number) => string;
  description?: string;
  defaultValue?: number;
  onReset?: () => void;
}) {
  const id = useId();
  return (
    <div className="avatar-range">
      <span>
        <label htmlFor={id}>{label}</label>
        <span className="control-value">
          <ResetValue
            label={label}
            value={value}
            defaultValue={defaultValue}
            onReset={onReset ?? (() => onChange(defaultValue!))}
          />
          <output htmlFor={id}>
            {format ? format(value) : Number(value.toFixed(3))}
          </output>
        </span>
      </span>
      <input
        id={id}
        aria-label={label}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      {description && <small>{description}</small>}
    </div>
  );
}
export function Choices<T extends string>({
  label,
  value,
  options,
  onChange,
  defaultValue,
  onReset,
}: {
  label: string;
  value: T;
  options: readonly (readonly [T, string])[];
  onChange: (value: T) => void;
  defaultValue?: T;
  onReset?: () => void;
}) {
  if (options.length > 4)
    return (
      <SelectControl
        {...{ label, value, options, onChange, defaultValue, onReset }}
      />
    );
  return (
    <fieldset className="study-choices">
      <legend>
        {label}
        <ResetValue
          label={label}
          value={value}
          defaultValue={defaultValue}
          onReset={onReset ?? (() => onChange(defaultValue!))}
        />
      </legend>
      <div>
        {options.map(([id, name]) => (
          <Button
            key={id}
            aria-pressed={value === id}
            onClick={() => onChange(id)}
          >
            {name}
          </Button>
        ))}
      </div>
    </fieldset>
  );
}
const percent = (v: number) => `${Math.round(v * 100)}%`;
const times = (v: number) => `${v.toFixed(2)}×`;

type Props = {
  config: SignalConfig;
  onChange: (v: Partial<SignalConfig>) => void;
  defaultConfig?: SignalConfig;
};
export function SelectControl<T extends string>({
  label,
  value,
  options,
  onChange,
  defaultValue,
  onReset,
}: {
  label: string;
  value: T;
  options: readonly (readonly [T, string])[];
  onChange: (value: T) => void;
  defaultValue?: T;
  onReset?: () => void;
}) {
  const id = useId();
  return (
    <div className="effect-select">
      <span>
        <label htmlFor={id}>{label}</label>
        <ResetValue
          label={label}
          value={value}
          defaultValue={defaultValue}
          onReset={onReset ?? (() => onChange(defaultValue!))}
        />
      </span>
      <select
        id={id}
        aria-label={label}
        value={value}
        onChange={(e) => onChange(e.target.value as T)}
      >
        {options.map(([id, name]) => (
          <option value={id} key={id}>
            {name}
          </option>
        ))}
      </select>
    </div>
  );
}
export function ControlGroup({
  title,
  note,
  children,
}: {
  title: string;
  note?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="effect-control-group">
      <h3>{title}</h3>
      {note && <p>{note}</p>}
      {children}
    </section>
  );
}
export function AvatarControls({
  config,
  onChange,
  section,
  defaultConfig,
}: Props & { motion?: boolean; focusCircle?: boolean; section?: string }) {
  return (
    <div className="character-properties">
      {(!section || section === 'Color') && (
        <ControlGroup title="Color">
          <SelectControl
            label="Base palette"
            value={config.color || 'mono'}
            defaultValue={
              defaultConfig ? defaultConfig.color || 'mono' : undefined
            }
            options={paletteOptions}
            onChange={(color) => onChange({ color, colorTiming: 'always' })}
          />
          <SelectControl
            label="Avatar accent"
            value={config.activityColor ?? 'mono'}
            defaultValue={
              defaultConfig
                ? (defaultConfig.activityColor ?? 'mono')
                : undefined
            }
            options={paletteOptions}
            onChange={(activityColor) =>
              onChange({ activityColor, colorTiming: 'always' })
            }
          />
          <RangeControl
            label="Base saturation"
            value={config.saturation ?? 0.45}
            defaultValue={
              defaultConfig ? (defaultConfig.saturation ?? 0.45) : undefined
            }
            min={0}
            max={1}
            step={0.05}
            format={percent}
            onChange={(saturation) => onChange({ saturation })}
            description="Zero is gray; higher values bring out hue."
          />
          <RangeControl
            label="Accent saturation"
            value={config.activitySaturation ?? 0.65}
            defaultValue={
              defaultConfig
                ? (defaultConfig.activitySaturation ?? 0.65)
                : undefined
            }
            min={0}
            max={1}
            step={0.05}
            format={percent}
            onChange={(activitySaturation) => onChange({ activitySaturation })}
          />
          <RangeControl
            label="Base strength"
            value={config.colorStrength ?? 0.8}
            defaultValue={
              defaultConfig ? (defaultConfig.colorStrength ?? 0.8) : undefined
            }
            min={0}
            max={1}
            step={0.05}
            format={percent}
            onChange={(colorStrength) => onChange({ colorStrength })}
          />
        </ControlGroup>
      )}
      {(!section || section === 'Field') && (
        <ControlGroup title="Field">
          <RangeControl
            label="Fill"
            value={config.fill ?? 0.5}
            defaultValue={
              defaultConfig ? (defaultConfig.fill ?? 0.5) : undefined
            }
            min={0}
            max={1}
            step={0.025}
            format={percent}
            onChange={(fill) => onChange({ fill })}
            description="Overall ink coverage, after the field is calculated."
          />
          <RangeControl
            label="Dome contribution"
            value={config.volume ?? 1}
            defaultValue={
              defaultConfig ? (defaultConfig.volume ?? 1) : undefined
            }
            min={0}
            max={2}
            step={0.05}
            format={times}
            onChange={(volume) => onChange({ volume })}
            description="Adds coverage according to height on a hemisphere: greatest at its center, zero at the edge."
          />
          <RangeControl
            label="Gradient strength"
            value={config.lightStrength ?? 1}
            defaultValue={
              defaultConfig ? (defaultConfig.lightStrength ?? 1) : undefined
            }
            min={0}
            max={2}
            step={0.05}
            format={times}
            onChange={(lightStrength) => onChange({ lightStrength })}
            description="The difference in coverage from one side to the other."
          />
          <RangeControl
            label="Gradient angle"
            value={config.lightAngle ?? 0}
            defaultValue={
              defaultConfig ? (defaultConfig.lightAngle ?? 0) : undefined
            }
            min={-180}
            max={180}
            step={5}
            format={(v) => `${v}°`}
            onChange={(lightAngle) => onChange({ lightAngle })}
          />
          <RangeControl
            label="Contrast"
            value={config.contrast ?? 1}
            defaultValue={
              defaultConfig ? (defaultConfig.contrast ?? 1) : undefined
            }
            min={0.5}
            max={2}
            step={0.05}
            format={times}
            onChange={(contrast) => onChange({ contrast })}
          />
          <RangeControl
            label="Vertical displacement"
            value={config.gravity ?? 0}
            defaultValue={
              defaultConfig ? (defaultConfig.gravity ?? 0) : undefined
            }
            min={0}
            max={1}
            step={0.05}
            format={percent}
            onChange={(gravity) => onChange({ gravity })}
            description="Displaces the interior downward."
          />
        </ControlGroup>
      )}
      {(!section || section === 'Print') && (
        <ControlGroup title="Print">
          <Choices
            label="Print method"
            value={config.texture}
            defaultValue={defaultConfig ? defaultConfig.texture : undefined}
            options={[
              ['bayer', 'Bayer'],
              ['halftone', 'Halftone'],
            ]}
            onChange={(texture) => onChange({ texture })}
          />
          <RangeControl
            label="Cell size"
            value={config.pitch}
            defaultValue={defaultConfig ? defaultConfig.pitch : undefined}
            min={0.8}
            max={3.5}
            step={0.1}
            format={(v) => `${v.toFixed(1)} px`}
            onChange={(pitch) => onChange({ pitch })}
            description="Spacing between printed marks."
          />
          {config.texture === 'bayer' && (
            <SelectControl
              label="Matrix"
              value={String(config.matrix ?? 8)}
              defaultValue={
                defaultConfig ? String(defaultConfig.matrix ?? 8) : undefined
              }
              options={[
                ['2', '2 × 2'],
                ['4', '4 × 4'],
                ['8', '8 × 8'],
              ]}
              onChange={(v) => onChange({ matrix: Number(v) as 2 | 4 | 8 })}
            />
          )}
          <SelectControl
            label="Tone bands"
            value={String(config.toneSteps ?? 0)}
            defaultValue={
              defaultConfig ? String(defaultConfig.toneSteps ?? 0) : undefined
            }
            options={[
              ['0', 'Continuous'],
              ['3', '3 tones'],
              ['5', '5 tones'],
              ['7', '7 tones'],
            ]}
            onChange={(v) => onChange({ toneSteps: Number(v) })}
          />
        </ControlGroup>
      )}
    </div>
  );
}
