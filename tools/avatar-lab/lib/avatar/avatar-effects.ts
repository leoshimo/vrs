import type { Expression } from './avatar-expressions';
// Effect names describe formulas. Expression names describe their particular use.
export const effects = {
  swirl: { name: 'Swirl', note: 'A loose current travels around the rim.' },
  orbit: {
    name: 'Orbit',
    note: 'A single crest circles the rim while working.',
  },
  entrance: { name: 'Entrance', note: 'Reveals the avatar.' },
  lava: {
    name: 'Lava',
    note: 'Changes the directional gradient and coverage over time.',
  },
  nudge: {
    name: 'Nudge',
    note: 'Briefly speeds up the field along its existing path, then settles.',
  },
  light: {
    name: 'Tilt',
    note: 'Briefly changes the gradient direction, then settles.',
  },
  agitation: {
    name: 'Stir',
    note: 'Crossing sine fields displace samples and perturb shading.',
  },
  'agitation-kick': {
    name: 'Ruffle',
    note: 'An event-seeded sine field briefly displaces samples and coverage.',
  },
  pressure: {
    name: 'Pressure',
    note: 'A localized radial displacement compresses the field.',
  },
  drift: {
    name: 'Drift',
    note: 'Translates the sampling field with a small periodic displacement.',
  },
  eddy: {
    name: 'Eddies',
    note: 'Rotates samples around a moving center with distance falloff.',
  },
  wave: {
    name: 'Sweep',
    note: 'A moving front displaces the field along its direction.',
  },
  surface: {
    name: 'Ripple',
    note: 'A ridge travels around the sphere and changes its outline, sampling and coverage.',
  },
  gather: {
    name: 'Gather',
    note: 'Compresses the field, then releases it with a decaying envelope.',
  },
  bloom: {
    name: 'Bloom',
    note: 'A pulse changes the interior coverage and color.',
  },
  sparks: {
    name: 'Sparks',
    note: 'Short marks radiate outside the silhouette.',
  },
  ripple: {
    name: 'Echo',
    note: 'A fading contour expands from the silhouette.',
  },
  fill: {
    name: 'Fill',
    note: 'Coverage rises from sparse marks to the configured fill.',
  },
  print: {
    name: 'Printed reveal',
    note: 'The field resolves from a sparse print into view.',
  },
} as const;
export type EffectId = keyof typeof effects;
export function expressionEffect(
  e: Pick<Expression, 'action' | 'settings'>,
): EffectId {
  if (e.action === 'idle') return e.settings.idleSwirl ? 'swirl' : 'lava';
  if (e.action === 'wake') return 'entrance';
  if (e.action === 'dispatch') return e.settings.avatarDispatch ?? 'gather';
  if (e.action === 'typing' && e.settings.typingMotion === 'nudge')
    return 'nudge';
  if (e.action === 'typing')
    return e.settings.typingMotion === 'agitation'
      ? 'agitation-kick'
      : e.settings.typingMotion === 'pressure'
        ? 'pressure'
        : 'light';
  if (e.settings.workingOrbit) return 'orbit';
  const motion = e.settings.loadingMotion;
  return motion === 'surface'
    ? 'surface'
    : motion === 'drift'
      ? 'drift'
      : motion === 'eddy'
        ? 'eddy'
        : motion === 'agitated'
          ? 'agitation'
          : motion === 'quicker'
            ? 'lava'
            : 'wave';
}
export const expressionUses = {
  idle: 'Idle',
  typing: 'Typing',
  loading: 'Working',
  dispatch: 'Submit',
  wake: 'Open',
} as const;

// Stable expression IDs retain stored overrides and saved presets.
const presetNames: Record<string, string> = {
  rest: 'Calm',
  fast: 'Faster',
  light: 'Tilt',
  'agitation-kick': 'Quick',
  pressure: 'Soft',
  drift: 'Oblique',
  eddies: 'Wandering',
  'gentle-agitation': 'Gentle',
  restless: 'Restless',
  alternating: 'Alternating',
  'right-left': 'From right',
  'left-right': 'From left',
  'top-bottom': 'Downward',
  'bottom-top': 'Upward',
  random: 'Wandering',
  'surface-working': 'Repeating',
  'surface-submit': 'Flourish',
  'surface-react': 'Short reaction',
  'field-nudge': 'Brief push',
  gather: 'Gather',
  bloom: 'Bloom',
  sparks: 'Sparks',
  echo: 'Small echo',
  'fill-wake': 'Fill',
  'print-wake': 'Printed reveal',
};
export function presetName(expression: Expression) {
  return presetNames[expression.id] ?? expression.name;
}
export function effectLibrary(library: Expression[]) {
  return (Object.keys(effects) as EffectId[])
    .map((id) => ({
      id,
      ...effects[id],
      presets: library.filter((e) => expressionEffect(e) === id),
    }))
    .filter((group) => group.presets.length);
}
