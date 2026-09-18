import { makeDefaultSetup, validateSetup, type Setup } from './setup';
import type { SignalConfig } from './signal-field';
import { withTypingStudies } from './typing-studies';
import { withLabStudies } from './lab-studies';

const tuning: Record<string, Partial<SignalConfig>> = {
  'surface-react': {
    surfaceAmplitude: 1,
    surfaceDuration: 2.5,
    surfaceOriginX: -0.4,
  },
  pressure: { typingEnergy: 1.2 },
  eddies: { loadingStrength: 1.85, loadingRate: 2.4, coupling: 1.7 },
  'gentle-agitation': {
    loadingStrength: 1.15,
    loadingRate: 0.3,
    coupling: 2.2,
  },
  restless: { loadingStrength: 0.55, loadingRate: 2.45 },
  'right-left': { loadingStrength: 1.05 },
  'surface-working': { surfaceAmplitude: 1, surfaceDuration: 2.4 },
  'surface-submit': {
    dispatchEnergy: 1.65,
    surfaceDuration: 3.4,
    surfaceWidth: 0.22,
    surfaceWavelength: 0.84,
    surfaceOriginX: -0.9,
    surfaceOriginY: -0.25,
  },
  sparks: { dispatchEnergy: 0.8, sparkCount: 11 },
  'fill-wake': { settle: 2, appearanceTarget: 'bar' },
  'print-wake': { appearanceTarget: 'bar' },
};

export function makeLabSetup(): Setup {
  const setup = makeDefaultSetup();
  setup.initialized = true;
  Object.assign(setup.appearance, {
    pitch: 2,
    loadingMotion: 'left-right',
    typingMotion: 'pressure',
    typingEnergy: 0.35,
    dispatchEnergy: 0.65,
    toneSteps: 3,
    colorStrength: 0.5,
    activityColor: 'opal',
    contrast: 1.65,
    saturation: 0.5,
    activitySaturation: 0.5,
    gravity: 1,
    dispatchTarget: 'selection',
    selectionRelease: 'press',
    language: 'porcelain',
  } satisfies Partial<SignalConfig>);
  for (const expression of setup.expressions) {
    for (const motion of expression.motions) {
      // Working keeps its quieter, slower Ripple alongside Stir.
      const patch =
        expression.id === 'working' && motion.preset === 'surface-working'
          ? undefined
          : tuning[motion.preset];
      Object.assign(motion.settings, patch);
      if (motion.preset === 'fill-wake') motion.duration = 0.3;
    }
  }
  return withLabStudies(withTypingStudies(setup));
}

export const draftKey = 'avatar-lab-draft-v1';

export function readLabDraft(raw: string | null): Setup {
  if (raw) {
    try {
      const setup: unknown = JSON.parse(raw);
      validateSetup(setup);
      return withLabStudies(withTypingStudies(setup));
    } catch {
      // An obsolete or damaged browser draft falls back to the code defaults.
    }
  }
  return makeLabSetup();
}
