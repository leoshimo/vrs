import { assignmentNames, type AvatarExpression, type AvatarSetup } from './expression-model';
import { effectSettings } from './effect-settings';
import { defaultSignal } from './signal-field';
import { paletteNames } from './expression-player';

const object = (v: unknown): v is Record<string, unknown> =>
  v !== null && typeof v === 'object' && !Array.isArray(v);
const number = (v: unknown, min: number, max: number): v is number =>
  typeof v === 'number' && Number.isFinite(v) && v >= min && v <= max;
const requiredAppearance = ['shape', 'placement', 'breathe', 'selection', 'texture',
  'effects', 'edge', 'amount', 'pitch', 'expression'];

export function isExpression(v: unknown): v is AvatarExpression {
  if (!object(v) || typeof v.id !== 'string' || typeof v.name !== 'string' ||
      typeof v.description !== 'string' || !number(v.duration, 0.01, 15) ||
      !Array.isArray(v.patterns) || !v.patterns.length || v.patterns.length > 8 ||
      !object(v.color)) return false;
  const c = v.color;
  return ['none', 'accent', 'custom'].includes(String(c.mode)) &&
    paletteNames.includes(String(c.palette)) && number(c.strength, 0, 1) &&
    number(c.attack, 0.001, 15) && number(c.duration, 0.01, 15) &&
    (v.attack === undefined || number(v.attack, 0.001, 15)) &&
    (v.revealStart === undefined || number(v.revealStart, 0, 1)) &&
    ['momentum', 'distortionOnly'].every(k => v[k] === undefined || typeof v[k] === 'boolean') &&
    v.patterns.every(p => {
      if (!object(p) || !Object.hasOwn(effectSettings, String(p.kind)) || !object(p.settings)) return false;
      const ranges = effectSettings[p.kind as keyof typeof effectSettings] as Record<string, readonly [string, number, number, number, number]>;
      return Object.entries(p.settings).every(([key, value]) => ranges[key] && number(value, ranges[key][1], ranges[key][2]));
    });
}

export function isAvatarSetup(v: unknown): v is AvatarSetup {
  if (!object(v) || !object(v.appearance) || !object(v.assignments)) return false;
  const appearance = v.appearance;
  // All renderer inputs must retain their types; optional fields may be omitted.
  if (!Object.entries(defaultSignal).every(([key, sample]) => {
    const value = appearance[key];
    if (value === undefined && !requiredAppearance.includes(key)) return true;
    if (typeof sample === 'number') return number(value, -1000, 1000);
    if (object(sample)) return object(value) && Object.keys(sample).every(k => typeof value[k] === 'boolean');
    return typeof value === typeof sample;
  })) return false;
  return assignmentNames.every(a => v.assignments && object(v.assignments) &&
    (v.assignments[a] === null || isExpression(v.assignments[a])));
}
