'use client';
import { Button } from '@/components/ui/button';
import { paletteOptions } from '@/lib/avatar/avatar-palettes';
import type { SignalConfig } from '@/lib/avatar/signal-field';
export type AvatarAction = 'idle' | 'typing' | 'loading' | 'dispatch' | 'wake';
export const avatarShapes = [
  ['pearl', 'Circle'],
  ['gentle', 'Living circle'],
  ['charged', 'Soft morph'],
] as const;
export function RangeControl({
  label,
  value,
  min,
  max,
  step = 0.01,
  onChange,
  format,
  description,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  format?: (v: number) => string;
  description?: string;
}) {
  return (
    <label className="avatar-range">
      <span>
        {label}
        <output>{format ? format(value) : value}</output>
      </span>
      <input
        aria-label={label}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      {description && <small>{description}</small>}
    </label>
  );
}
export function Choices<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly (readonly [T, string])[];
  onChange: (v: T) => void;
}) {
  if (options.length > 4)
    return (
      <SelectControl
        label={label}
        value={value}
        options={options}
        onChange={onChange}
      />
    );
  return (
    <fieldset className="study-choices">
      <legend>{label}</legend>
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
};
export function AvatarDispatchControls({ config, onChange }: Props) {
  return (
    <Choices
      label="Expression"
      value={config.avatarDispatch || 'gather'}
      options={[
        ['gather', 'Gather'],
        ['bloom', 'Bloom'],
        ['sparks', 'Sparks'],
        ['ripple', 'Small echo'],
        ['surface', 'Surface wave'],
      ]}
      onChange={(avatarDispatch) => onChange({ avatarDispatch })}
    />
  );
}
export function SelectControl<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly (readonly [T, string])[];
  onChange: (value: T) => void;
}) {
  return (
    <label className="effect-select">
      <span>{label}</span>
      <select
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
    </label>
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
}: Props & { motion?: boolean; focusCircle?: boolean; section?: string }) {
  return (
    <div className="character-properties">
      {(!section || section === 'Color') && (
        <ControlGroup title="Color">
          <SelectControl
            label="Base palette"
            value={config.color || 'mono'}
            options={paletteOptions}
            onChange={(color) => onChange({ color, colorTiming: 'always' })}
          />
          <SelectControl
            label="Avatar accent"
            value={config.activityColor ?? 'mono'}
            options={paletteOptions}
            onChange={(activityColor) =>
              onChange({ activityColor, colorTiming: 'always' })
            }
          />
          <RangeControl
            label="Base saturation"
            value={config.saturation ?? 0.45}
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
            min={0}
            max={1}
            step={0.05}
            format={percent}
            onChange={(activitySaturation) => onChange({ activitySaturation })}
          />
          <RangeControl
            label="Base strength"
            value={config.colorStrength ?? 0.8}
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
            min={-180}
            max={180}
            step={5}
            format={(v) => `${v}°`}
            onChange={(lightAngle) => onChange({ lightAngle })}
          />
          <RangeControl
            label="Contrast"
            value={config.contrast ?? 1}
            min={0.5}
            max={2}
            step={0.05}
            format={times}
            onChange={(contrast) => onChange({ contrast })}
          />
          <RangeControl
            label="Vertical displacement"
            value={config.gravity ?? 0}
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
            options={[
              ['bayer', 'Bayer'],
              ['halftone', 'Halftone'],
            ]}
            onChange={(texture) => onChange({ texture })}
          />
          <RangeControl
            label="Cell size"
            value={config.pitch}
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
export function ExpressionControls({
  action,
  config,
  onChange,
  hideVariant = false,
}: Props & { action: AvatarAction; hideVariant?: boolean }) {
  return (
    <div className="expression-properties">
      {config.idleSwirl || config.workingOrbit ? (
        <>
          <RangeControl
            label="Rim strength"
            value={config.idleSwirl || config.workingOrbit || 1}
            min={0.1}
            max={2}
            step={0.05}
            onChange={(value) =>
              onChange(
                config.idleSwirl
                  ? { idleSwirl: value }
                  : { workingOrbit: value },
              )
            }
          />
          <RangeControl
            label="Playback rate"
            value={
              config.idleSwirl
                ? (config.swirlRate ?? 0.8)
                : (config.orbitRate ?? 1)
            }
            min={0.2}
            max={3}
            step={0.1}
            onChange={(value) =>
              onChange(
                config.idleSwirl ? { swirlRate: value } : { orbitRate: value },
              )
            }
          />
        </>
      ) : null}
      {action === 'idle' && (
        <>
          <RangeControl
            label="Flow amplitude"
            value={config.idleAmount ?? 1.2}
            min={0.3}
            max={2.4}
            step={0.1}
            format={times}
            onChange={(idleAmount) => onChange({ idleAmount })}
            description="Scales the moving density variation and slightly raises its rate."
          />
        </>
      )}
      {action === 'typing' && (
        <>
          {!hideVariant && (
            <Choices
              label="Expression"
              value={config.typingMotion || 'light'}
              options={[
                ['light', 'Light kick'],
                ['agitation', 'Agitation kick'],
                ['pressure', 'Soft pressure'],
                ['nudge', 'Field nudge'],
              ]}
              onChange={(typingMotion) =>
                onChange({
                  typingMotion,
                  ...(typingMotion === 'pressure'
                    ? { typingEnergy: 0.35 }
                    : {}),
                })
              }
            />
          )}
          <RangeControl
            label="Impulse amplitude"
            value={config.typingEnergy ?? 1.15}
            min={0.1}
            max={2.5}
            step={0.01}
            format={times}
            onChange={(typingEnergy) => onChange({ typingEnergy })}
            description={
              config.typingMotion === 'nudge'
                ? 'Each trigger adds forward speed to the field clock; the extra speed gradually falls away.'
                : 'Each trigger pushes the motion; repeated triggers retain its current movement.'
            }
          />
          {config.typingMotion === 'pressure' && (
            <>
              <RangeControl
                label="Concentration"
                value={config.pressureSpread ?? 3}
                min={0.5}
                max={10}
                step={0.1}
                onChange={(pressureSpread) => onChange({ pressureSpread })}
              />
              <RangeControl
                label="Distance from center"
                value={config.pressureOffset ?? 0.36}
                min={0}
                max={0.95}
                step={0.01}
                onChange={(pressureOffset) => onChange({ pressureOffset })}
              />
            </>
          )}
        </>
      )}
      {action === 'loading' && (
        <>
          {!hideVariant && (
            <Choices
              label="Expression"
              value={config.loadingMotion || 'alternating'}
              options={[
                ['alternating', 'Alternating'],
                ['right-left', 'Right → left'],
                ['left-right', 'Left → right'],
                ['top-bottom', 'Downward'],
                ['bottom-top', 'Upward'],
                ['random', 'Wandering'],
                ['drift', 'Drift'],
                ['quicker', 'Fast idle'],
                ['agitated', 'Agitation'],
                ['eddy', 'Eddies'],
                ['surface', 'Surface wave'],
              ]}
              onChange={(loadingMotion) => onChange({ loadingMotion })}
            />
          )}
          {config.loadingMotion === 'surface' ? (
            <SurfaceWaveControls config={config} onChange={onChange} />
          ) : (
            <>
              <RangeControl
                label={
                  config.loadingMotion === 'quicker'
                    ? 'Activity strength'
                    : 'Deformation amplitude'
                }
                value={config.loadingStrength ?? 1}
                min={0.15}
                max={2}
                step={0.05}
                format={times}
                onChange={(loadingStrength) => onChange({ loadingStrength })}
                description={
                  config.loadingMotion === 'quicker'
                    ? 'Flattens and tilts the directional gradient during activity.'
                    : 'From a light disturbance to a more restless field.'
                }
              />
              <RangeControl
                label="Playback rate"
                value={config.loadingRate ?? 1}
                min={0.25}
                max={2.5}
                step={0.05}
                format={times}
                onChange={(loadingRate) => onChange({ loadingRate })}
              />
              {config.loadingMotion === 'drift' && (
                <RangeControl
                  label="Drift direction"
                  value={config.driftAngle ?? 0}
                  min={-180}
                  max={180}
                  step={5}
                  format={(v) => `${v}°`}
                  onChange={(driftAngle) => onChange({ driftAngle })}
                  description="0° travels horizontally; 90° travels vertically."
                />
              )}
              {config.loadingMotion === 'radial' && (
                <RangeControl
                  label="Wave origin"
                  value={config.waveOffset ?? 0}
                  min={-0.5}
                  max={0.5}
                  step={0.05}
                  format={(v) => (v === 0 ? 'Center' : `${v.toFixed(2)}`)}
                  onChange={(waveOffset) => onChange({ waveOffset })}
                  description="Moves the pull away from the center."
                />
              )}
              {config.loadingMotion !== 'quicker' && (
                <>
                  <RangeControl
                    label="Deformation gain"
                    value={config.coupling ?? 1}
                    min={0.3}
                    max={2.5}
                    step={0.1}
                    format={times}
                    onChange={(coupling) => onChange({ coupling })}
                    description="Scales distortion and its influence on shading."
                  />
                </>
              )}
              {!['drift', 'eddy', 'agitated', 'quicker'].includes(
                config.loadingMotion ?? '',
              ) && (
                <>
                  <Choices
                    label="Wave color"
                    value={config.rippleAccent ? 'accent' : 'shared'}
                    options={[
                      ['shared', 'Shared ink'],
                      ['accent', 'Accent'],
                    ]}
                    onChange={(v) => onChange({ rippleAccent: v === 'accent' })}
                  />
                </>
              )}
            </>
          )}
        </>
      )}
      {action === 'dispatch' && (
        <>
          {!hideVariant && (
            <AvatarDispatchControls config={config} onChange={onChange} />
          )}
          {config.avatarDispatch === 'surface' && (
            <SurfaceWaveControls config={config} onChange={onChange} />
          )}
          <RangeControl
            label="Impulse amplitude"
            value={config.dispatchEnergy ?? 1.2}
            min={0.3}
            max={2.5}
            step={0.05}
            format={times}
            onChange={(dispatchEnergy) => onChange({ dispatchEnergy })}
          />
          {config.avatarDispatch === 'sparks' && (
            <RangeControl
              label="Spark count"
              value={config.sparkCount ?? 10}
              min={4}
              max={18}
              step={1}
              onChange={(sparkCount) => onChange({ sparkCount })}
            />
          )}
        </>
      )}
      {action === 'wake' && !hideVariant && (
        <Choices
          label="Entrance"
          value={config.appearanceMode ?? 'fill'}
          options={[
            ['fill', 'Fill'],
            ['print', 'Printed reveal'],
          ]}
          onChange={(appearanceMode) => onChange({ appearanceMode })}
        />
      )}
      {action === 'wake' && config.entranceRipple && (
        <RangeControl
          label="Ripple strength"
          value={config.surfaceAmplitude ?? 0.45}
          min={0}
          max={1}
          step={0.05}
          onChange={(surfaceAmplitude) => onChange({ surfaceAmplitude })}
        />
      )}
      {action === 'dispatch' && (
        <RangeControl
          label="Decay scale"
          value={config.settle ?? 1.5}
          min={0.5}
          max={3}
          step={0.1}
          format={times}
          onChange={(settle) => onChange({ settle })}
          description="How long the interior coasts after the gesture."
        />
      )}
    </div>
  );
}

export function SurfaceWaveControls({
  config,
  onChange,
}: {
  config: SignalConfig;
  onChange: (v: Partial<SignalConfig>) => void;
}) {
  return (
    <>
      <RangeControl
        label="Surface amplitude"
        value={config.surfaceAmplitude ?? 0.8}
        min={0}
        max={1.8}
        step={0.05}
        onChange={(surfaceAmplitude) => onChange({ surfaceAmplitude })}
        description="Scales the ridge and its effect on the interior pattern."
      />
      <RangeControl
        label="Wave duration"
        value={config.surfaceDuration ?? 3}
        min={0.6}
        max={8}
        step={0.1}
        format={(v) => `${v.toFixed(1)} s`}
        onChange={(surfaceDuration) => onChange({ surfaceDuration })}
        description="Time for one wave to travel across the sphere."
      />
      <RangeControl
        label="Origin · horizontal"
        description="Moves the wave’s starting point across the sphere."
        value={config.surfaceOriginX ?? -0.3}
        min={-0.9}
        max={0.9}
        step={0.05}
        onChange={(surfaceOriginX) => onChange({ surfaceOriginX })}
      />
      <RangeControl
        label="Origin · vertical"
        value={config.surfaceOriginY ?? -0.45}
        min={-0.9}
        max={0.9}
        step={0.05}
        onChange={(surfaceOriginY) => onChange({ surfaceOriginY })}
      />
      <RangeControl
        label="Envelope width"
        format={(v) => `${v.toFixed(2)} rad`}
        description="How broad the traveling patch is, measured around the sphere."
        value={config.surfaceWidth ?? 0.32}
        min={0.12}
        max={0.8}
        step={0.02}
        onChange={(surfaceWidth) => onChange({ surfaceWidth })}
      />
      <ControlGroup
        title="Wave"
        note="The spacing and decay of the ridge as it travels around the sphere."
      >
        <RangeControl
          label="Wavelength"
          format={(v) => `${v.toFixed(2)} rad`}
          description="Distance between neighboring crests, in radians around the sphere."
          value={config.surfaceWavelength ?? Math.PI / 6}
          min={0.2}
          max={1.2}
          step={0.02}
          onChange={(surfaceWavelength) => onChange({ surfaceWavelength })}
        />
        <RangeControl
          label="Spatial damping"
          description="Makes the wave lose height as it travels away from its origin."
          value={config.surfaceDamping ?? 0}
          min={0}
          max={2}
          step={0.05}
          onChange={(surfaceDamping) => onChange({ surfaceDamping })}
        />
        {(
          [
            ['surfaceSilhouette', 'Silhouette'],
            ['surfaceDisplacement', 'Interior displacement'],
            ['surfaceShading', 'Coverage'],
          ] as const
        ).map(([key, label]) => (
          <RangeControl
            key={key}
            label={label}
            description={
              key === 'surfaceShading'
                ? 'How strongly the ridge changes the amount of ink.'
                : undefined
            }
            value={config[key] ?? 1}
            min={0}
            max={1.5}
            step={0.05}
            onChange={(v) => onChange({ [key]: v })}
          />
        ))}
      </ControlGroup>
    </>
  );
}
