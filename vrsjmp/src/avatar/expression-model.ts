import type { SignalConfig, SignalColor } from "./signal-field";

export const assignmentNames = [
  "idle",
  "typing",
  "working",
  "open",
  "openAfterIdle",
  "complete",
] as const;
export type Assignment = (typeof assignmentNames)[number];
export const assignmentLabels: Record<Assignment, string> = {
  idle: "Idle",
  typing: "Typing",
  working: "Working",
  open: "Open",
  openAfterIdle: "Open after idle",
  complete: "Complete",
};
export type MotionKind =
  | "lava"
  | "swirl"
  | "orbit"
  | "nudge"
  | "tilt"
  | "pressure"
  | "ruffle"
  | "stir"
  | "drift"
  | "eddies"
  | "sweep"
  | "ripple"
  | "gather"
  | "bloom"
  | "sparks"
  | "echo"
  | "printed"
  | "fill";
export type Pattern = { kind: MotionKind; settings: Partial<SignalConfig> };
export type ExpressionColor = {
  mode: "none" | "accent" | "custom";
  palette: SignalColor;
  strength: number;
  attack: number;
  duration: number;
};
export type AvatarExpression = {
  id: string;
  name: string;
  description: string;
  patterns: Pattern[];
  duration: number;
  revealStart?: number;
  color: ExpressionColor;
  momentum?: boolean;
  distortionOnly?: boolean;
  custom?: boolean;
};
export type AvatarSetup = {
  version: 2;
  appearance: SignalConfig;
  expressions: AvatarExpression[];
  assignments: Record<Assignment, string | null>;
};
export const motionNames: Record<MotionKind, string> = {
  lava: "Lava",
  swirl: "Swirl",
  orbit: "Orbit",
  nudge: "Nudge",
  tilt: "Tilt",
  pressure: "Pressure",
  ruffle: "Ruffle",
  stir: "Stir",
  drift: "Drift",
  eddies: "Eddies",
  sweep: "Sweep",
  ripple: "Ripple",
  gather: "Gather",
  bloom: "Bloom",
  sparks: "Sparks",
  echo: "Echo",
  printed: "Printed",
  fill: "Fill",
};
export const defaultColor: ExpressionColor = {
  mode: "accent",
  palette: "opal",
  strength: 0.35,
  attack: 0.07,
  duration: 0.32,
};
export function assigned(setup: AvatarSetup, assignment: Assignment) {
  return setup.expressions.find((e) => e.id === setup.assignments[assignment]);
}
export function previewAssignment(expression: AvatarExpression): Assignment {
  if (expression.patterns.some((p) => p.kind === "printed" || p.kind === "fill"))
    return "openAfterIdle";
  if (expression.patterns.some((p) => ["nudge", "tilt", "pressure", "ruffle"].includes(p.kind)))
    return "typing";
  if (expression.patterns.some((p) => ["sparks", "echo", "bloom", "gather"].includes(p.kind)))
    return "complete";
  return "working";
}
export function remix(
  setup: AvatarSetup,
  id: string,
  newId: string,
  assignment?: Assignment,
): AvatarSetup {
  const source = setup.expressions.find((e) => e.id === id);
  if (!source) return setup;
  return {
    ...setup,
    expressions: [
      ...setup.expressions,
      { ...structuredClone(source), id: newId, name: `${source.name} — Remix`, custom: true },
    ],
    assignments: assignment ? { ...setup.assignments, [assignment]: newId } : setup.assignments,
  };
}
export function removeExpression(setup: AvatarSetup, id: string): AvatarSetup {
  return {
    ...setup,
    expressions: setup.expressions.filter((e) => e.id !== id),
    assignments: Object.fromEntries(
      assignmentNames.map((a) => [a, setup.assignments[a] === id ? null : setup.assignments[a]]),
    ) as AvatarSetup["assignments"],
  };
}
