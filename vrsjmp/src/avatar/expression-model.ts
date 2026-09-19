import type { SignalConfig, SignalColor } from "./signal-field";

export const assignmentNames = [
  "idle",
  "typing",
  "submit",
  "working",
  "open",
  "openAfterIdle",
  "complete",
] as const;
export type Assignment = (typeof assignmentNames)[number];
export const assignmentLabels: Record<Assignment, string> = {
  idle: "Idle",
  typing: "Typing",
  submit: "Submit",
  working: "Working",
  open: "Open",
  openAfterIdle: "Open after idle",
  complete: "Complete",
};
export { type MotionKind, type Pattern } from "./effect-settings";
import type { MotionKind, Pattern } from "./effect-settings";
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
  attack?: number;
  revealStart?: number;
  color: ExpressionColor;
  momentum?: boolean;
  distortionOnly?: boolean;
};
export type AvatarSetup = {
  appearance: SignalConfig;
  assignments: Record<Assignment, AvatarExpression | null>;
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
  return setup.assignments[assignment] ?? undefined;
}
