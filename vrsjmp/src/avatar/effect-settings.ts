// Each effect declares only the parameters it consumes. The lab uses the same ranges.
type Parameter = readonly [label: string, min: number, max: number, step: number, initial: number];
const moving = {
  strength: ["Strength", 0, 2.5, 0.05, 1],
  speed: ["Speed", 0, 3, 0.05, 1],
  distortion: ["Distortion", 0, 3, 0.1, 1],
} as const;
export const effectSettings = {
  lava: {
    flowStrength: ["Flow strength", 0, 2.4, 0.1, 1.6],
    flowSpeed: ["Flow speed", 0, 3, 0.05, 1],
    lightRotation: ["Light rotation", 0, 3, 0.05, 1],
  },
  nudge: { strength: ["Strength", 0, 1.5, 0.05, 0.6] },
  tilt: { strength: ["Strength", 0, 2, 0.05, 1] },
  pressure: {
    strength: ["Strength", 0, 1.5, 0.05, 0.28],
    spread: ["Concentration", 0.5, 10, 0.1, 3],
    offset: ["Distance from center", 0, 0.95, 0.05, 0.36],
  },
  ruffle: { strength: ["Strength", 0, 2, 0.05, 0.5] },
  stir: moving,
  drift: { ...moving, angle: ["Direction", -180, 180, 1, 20] },
  eddies: moving,
  sweep: { ...moving, direction: ["Direction", 0, 4, 1, 0] },
  swirl: { strength: ["Strength", 0, 2, 0.05, 0.8], speed: ["Speed", 0, 3, 0.1, 0.8] },
  orbit: { strength: ["Strength", 0, 2, 0.05, 1], speed: ["Speed", 0, 3, 0.1, 1.8] },
  ripple: {
    strength: ["Strength", 0, 1.5, 0.05, 0.45],
    travel: ["Travel time", 0.15, 15, 0.05, 0.65],
    width: ["Width", 0.1, 0.8, 0.02, 0.32],
    originX: ["Origin X", -1, 1, 0.05, -0.7],
    originY: ["Origin Y", -1, 1, 0.05, -0.6],
    silhouette: ["Silhouette", 0, 2, 0.05, 1],
    displacement: ["Displacement", 0, 2, 0.05, 1],
    shading: ["Shading", 0, 2, 0.05, 1],
  },
  gather: { strength: ["Strength", 0, 2, 0.1, 1] },
  bloom: { strength: ["Strength", 0, 2, 0.1, 0.8] },
  sparks: { strength: ["Strength", 0, 2, 0.1, 0.8], count: ["Count", 3, 18, 1, 11] },
  echo: { strength: ["Strength", 0, 2, 0.1, 1] },
  printed: {},
  fill: {},
} as const satisfies Record<string, Record<string, Parameter>>;
export type MotionKind = keyof typeof effectSettings;
export type Pattern = {
  [K in MotionKind]: {
    kind: K;
    settings: Partial<Record<keyof (typeof effectSettings)[K], number>>;
  };
}[MotionKind];
export function parameters(pattern: Pattern): Record<string, number> {
  return Object.fromEntries(
    Object.entries(effectSettings[pattern.kind]).map(([key, spec]) => [
      key,
      (pattern.settings as Record<string, number>)[key] ?? spec[4],
    ]),
  );
}
export function readPattern(kind: MotionKind, raw: Record<string, unknown>): Pattern {
  const settings: Record<string, number> = {};
  for (const [key, spec] of Object.entries(effectSettings[kind])) {
    const value = raw[key];
    if (typeof value === "number" && Number.isFinite(value))
      settings[key] = Math.max(spec[1], Math.min(spec[2], value));
  }
  return { kind, settings } as Pattern;
}
