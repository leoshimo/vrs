import type { SignalConfig } from "./signal-field.ts";
export type AvatarAction = "idle" | "typing" | "loading" | "dispatch" | "wake";
export type Expression = {
  id: string;
  name: string;
  action: AvatarAction;
  note: string;
  settings: Partial<SignalConfig>;
  duration?: number;
};
export const expressions: Expression[] = [
  {
    id: "rest",
    name: "Lava",
    action: "idle",
    note: "Slow, continuous changes in light.",
    settings: { idleAmount: 1.6 },
  },
  {
    id: "field-nudge",
    name: "Nudge",
    action: "typing",
    note: "Briefly accelerates the field, then returns to its normal pace.",
    settings: { typingMotion: "nudge", typingEnergy: 0.6 },
  },
  {
    id: "surface-react",
    name: "Quick ripple",
    action: "dispatch",
    note: "A small, quick ridge for a brief reaction.",
    settings: {
      avatarDispatch: "surface",
      surfaceAmplitude: 0.25,
      surfaceDuration: 0.9,
      dispatchEnergy: 0.3,
      settle: 0.7,
    },
  },
  {
    id: "light",
    name: "Tilt",
    action: "typing",
    note: "A quick change of illumination.",
    settings: { typingMotion: "light", typingEnergy: 1.15 },
  },
  {
    id: "agitation-kick",
    name: "Ruffle",
    action: "typing",
    note: "A brief ruffle through the interior.",
    settings: { typingMotion: "agitation", typingEnergy: 1 },
  },
  {
    id: "pressure",
    name: "Soft pressure",
    action: "typing",
    note: "A small compression, then release.",
    settings: { typingMotion: "pressure", typingEnergy: 0.35 },
  },
  {
    id: "drift",
    name: "Oblique drift",
    action: "loading",
    note: "A steady current at a slight angle.",
    settings: {
      loadingMotion: "drift",
      driftAngle: 20,
      loadingStrength: 1,
      loadingRate: 1,
    },
  },
  {
    id: "eddies",
    name: "Eddies",
    action: "loading",
    note: "A wandering, rotating pocket.",
    settings: { loadingMotion: "eddy", loadingStrength: 1, loadingRate: 1 },
  },
  {
    id: "gentle-agitation",
    name: "Gentle stir",
    action: "loading",
    note: "Soft irregular movement.",
    settings: {
      loadingMotion: "agitated",
      loadingStrength: 0.4,
      loadingRate: 0.8,
    },
  },
  {
    id: "restless",
    name: "Restless stir",
    action: "loading",
    note: "A stronger, faster disturbance.",
    settings: {
      loadingMotion: "agitated",
      loadingStrength: 1.6,
      loadingRate: 1.4,
    },
  },
  {
    id: "fast",
    name: "Fast idle",
    action: "loading",
    note: "The familiar field, moving faster.",
    settings: { loadingMotion: "quicker", loadingStrength: 1, loadingRate: 1 },
  },
  {
    id: "alternating",
    name: "Alternating sweep",
    action: "loading",
    note: "A wave arrives from either side.",
    settings: {
      loadingMotion: "alternating",
      loadingStrength: 1,
      loadingRate: 1,
    },
  },
  {
    id: "right-left",
    name: "From the right",
    note: "A single wave crosses from right to left.",
    action: "loading",
    settings: {
      loadingMotion: "right-left",
      loadingStrength: 1,
      loadingRate: 1,
    },
  },
  {
    id: "left-right",
    name: "From the left",
    note: "A single wave crosses from left to right.",
    action: "loading",
    settings: {
      loadingMotion: "left-right",
      loadingStrength: 1,
      loadingRate: 1,
    },
  },
  {
    id: "top-bottom",
    name: "Downpour",
    note: "A traveling disturbance moves downward.",
    action: "loading",
    settings: {
      loadingMotion: "top-bottom",
      loadingStrength: 1,
      loadingRate: 1,
    },
  },
  {
    id: "bottom-top",
    name: "Updraft",
    note: "A traveling disturbance rises from below.",
    action: "loading",
    settings: {
      loadingMotion: "bottom-top",
      loadingStrength: 1,
      loadingRate: 1,
    },
  },
  {
    id: "random",
    name: "Wandering wave",
    note: "Each arriving wave chooses a new direction.",
    action: "loading",
    settings: { loadingMotion: "random", loadingStrength: 1, loadingRate: 1 },
  },
  {
    id: "surface-working",
    name: "Repeating ripple",
    action: "loading",
    note: "A ridge travels over the sphere, repeating smoothly.",
    settings: {
      loadingMotion: "surface",
      surfaceAmplitude: 0.8,
      surfaceDuration: 4,
      loadingStrength: 1,
    },
  },
  {
    id: "surface-submit",
    name: "Ripple",
    action: "dispatch",
    note: "One ridge crosses the sphere and settles.",
    settings: {
      avatarDispatch: "surface",
      surfaceAmplitude: 0.8,
      surfaceDuration: 2.4,
      dispatchEnergy: 0.65,
    },
  },
  {
    id: "gather",
    name: "Gather",
    action: "dispatch",
    note: "Draw inward, then coast.",
    settings: { avatarDispatch: "gather", dispatchEnergy: 1.2 },
  },
  {
    id: "bloom",
    name: "Bloom",
    action: "dispatch",
    note: "An internal flash of pigment.",
    settings: { avatarDispatch: "bloom", dispatchEnergy: 1.2 },
  },
  {
    id: "sparks",
    name: "Sparks",
    action: "dispatch",
    note: "Short marks escape the surface.",
    settings: { avatarDispatch: "sparks", sparkCount: 14 },
  },
  {
    id: "echo",
    name: "Echo",
    note: "A short contour follows the shape, then fades.",
    action: "dispatch",
    settings: { avatarDispatch: "ripple", dispatchEnergy: 1 },
  },
  {
    id: "fill-wake",
    name: "Fill wake",
    action: "wake",
    note: "Sparse ink reaches its resting fill.",
    settings: { appearanceMode: "fill" },
  },
  {
    id: "print-wake",
    name: "Printed reveal",
    action: "wake",
    note: "The material resolves into view.",
    settings: { appearanceMode: "print" },
  },
];
const surfaceKeys: (keyof SignalConfig)[] = [
  "surfaceAmplitude",
  "surfaceDuration",
  "surfaceWidth",
  "surfaceWavelength",
  "surfaceDamping",
  "surfaceOriginX",
  "surfaceOriginY",
  "surfaceSilhouette",
  "surfaceDisplacement",
  "surfaceShading",
];
const keys: Record<AvatarAction, (keyof SignalConfig)[]> = {
  idle: ["idleAmount"],
  typing: ["typingMotion", "typingEnergy"],
  loading: [
    "loadingMotion",
    "loadingStrength",
    "loadingRate",
    "driftAngle",
    "waveOffset",
    "coupling",
    "rippleAccent",
    ...surfaceKeys,
  ],
  dispatch: ["avatarDispatch", "dispatchEnergy", "sparkCount", "settle", ...surfaceKeys],
  wake: ["appearanceMode", "settle", "appearanceTarget"],
};
export function expressionSettings(
  action: AvatarAction,
  config: SignalConfig,
): Partial<SignalConfig> {
  return Object.fromEntries(keys[action].map((k) => [k, config[k]]));
}
