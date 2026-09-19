import type { SignalConfig } from "./signal-field";
import { paletteNames, type Uniforms } from "./expression-player";
export function appearanceUniforms(config: SignalConfig, dark: boolean): Uniforms {
  return {
    u_dark: Number(dark),
    u_breathe: Number(config.breathe),
    u_texture: Number(config.texture === "halftone"),
    u_pitch: config.pitch,
    u_expression: config.expression,
    u_color: paletteNames.indexOf(config.color ?? "mono"),
    u_saturation: config.saturation ?? 0.45,
    u_activitySaturation: config.activitySaturation ?? 0.65,
    u_colorStrength: config.colorStrength ?? 0.5,
    u_fill: config.fill ?? 0.5,
    u_contrast: config.contrast ?? 1,
    u_matrix: config.matrix ?? 8,
    u_volume: config.volume ?? 1,
    u_lightStrength: config.lightStrength ?? 1,
    u_lightAngle: ((config.lightAngle ?? 0) * Math.PI) / 180,
    u_toneSteps: config.toneSteps ?? 0,
    u_debugStage: config.debugStage ?? 0,
    u_print: Number(config.effects.indicator),
    u_recipeDisplacement: [
      config.primitiveDrift ?? 0,
      config.primitiveTwist ?? 0,
      config.primitiveNoise ?? 0,
      config.gravity ?? 0,
    ],
    u_recipePoke: config.inspectPoke ?? [0, 0],
  };
}
