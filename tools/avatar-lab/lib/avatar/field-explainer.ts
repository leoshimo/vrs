import { pulseEnvelope } from '../../../../vrsjmp/src/avatar/response-curve.ts';
// Same rank order and half-step normalization as printed() in signal-field.ts.
export function bayerThreshold(x: number, y: number, size: number) {
  const b2 = (a: number, b: number) =>
    2 * (a % 2) + 3 * (b % 2) - 4 * (a % 2) * (b % 2);
  const rank =
    size === 2
      ? b2(x, y)
      : size === 4
        ? 4 * b2(x, y) + b2(Math.floor(x / 2), Math.floor(y / 2))
        : 16 * b2(x, y) +
          4 * b2(Math.floor(x / 2), Math.floor(y / 2)) +
          b2(Math.floor(x / 4), Math.floor(y / 4));
  return (rank + 0.5) / (size * size);
}
export function tutorialDensity(
  x: number,
  y: number,
  volume: number,
  angle: number,
  strength: number,
  fill: number,
  stage: number,
) {
  let value = 0.4;
  if (stage > 1)
    value += 0.25 * Math.sqrt(Math.max(0, 1 - x * x - y * y)) * volume;
  if (stage > 2)
    value +=
      0.34 *
      (x * Math.cos((angle * Math.PI) / 180) +
        y * Math.sin((angle * Math.PI) / 180)) *
      strength;
  value = Math.max(0.015, Math.min(0.985, value));
  return fill < 0.5 ? value * fill * 2 : value + (1 - value) * (fill - 0.5) * 2;
}

export type CoordinateOperation =
  | 'translation'
  | 'twist'
  | 'shear'
  | 'vertical';
const smooth = (a: number, b: number, value: number) => {
  const t = Math.max(0, Math.min(1, (value - a) / (b - a)));
  return t * t * (3 - 2 * t);
};
// Exact displacement terms from the coordinate-demonstration branch of signalFragment, in avatar-radius units.
export function sampleDisplacement(
  x: number,
  y: number,
  phase: number,
  amount: number,
  operation: CoordinateOperation,
): [number, number] {
  if (operation === 'translation')
    return [
      amount * 0.24 * Math.sin(phase * 0.9),
      amount * 0.24 * Math.cos(phase * 0.67),
    ];
  if (operation === 'shear')
    return [
      amount * 0.19 * Math.sin(y * 4 + phase * 2.7),
      amount * 0.19 * Math.cos(x * 3.4 - phase * 2.1),
    ];
  if (operation === 'vertical')
    return [0, amount * 0.3 * (1 - smooth(0.15, 1.1, Math.hypot(x, y)))];
  const qx = x - 0.2 * Math.sin(phase),
    qy = y - 0.2 * Math.cos(phase * 0.83);
  const weight =
    amount *
    Math.exp(-(qx * qx + qy * qy) * 1.8) *
    Math.sin(phase * 1.6) *
    0.65;
  return [-qy * weight, qx * weight];
}
export function paletteCoordinate(x: number, y: number, phase: number) {
  return 0.5 + 0.5 * Math.sin(x * 1.8 + y * 0.9 + phase * 0.7);
}
export function toneBand(density: number, count: number) {
  return count > 1
    ? Math.floor(Math.max(0, Math.min(0.999, density)) * count) / (count - 1)
    : density;
}
export function palettePigment(
  t: number,
  instrument = false,
  saturation = 0.45,
  dark = false,
): number[] {
  const [a, b, c] = instrument
    ? [
        [0.18, 0.52, 0.89],
        [0.52, 0.77, 0.6],
        [0.97, 0.59, 0.27],
      ]
    : [
        [0.52, 0.68, 0.9],
        [0.76, 0.61, 0.83],
        [0.93, 0.8, 0.61],
      ];
  const u = t < 0.5 ? smooth(0, 0.5, t) : smooth(0.5, 1, t);
  const start = t < 0.5 ? a : b,
    end = t < 0.5 ? b : c;
  const rgb = start.map((v, i) => v + (end[i] - v) * u);
  const gray = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
  return rgb.map(
    (v) =>
      (gray + (v - gray) * saturation) * (dark ? 1 : 0.7) + (dark ? 0.12 : 0),
  );
}
export const rgbCSS = (rgb: number[]) =>
  `rgb(${rgb.map((v) => Math.round(Math.max(0, Math.min(1, v)) * 255)).join(' ')})`;

export function rippleAtAngle(
  angle: number,
  phase: number,
  amplitude: number,
  travel = 3,
) {
  const time = (phase / 4) * travel;
  const delay = (angle / Math.PI) * travel;
  const pulseDuration = Math.max(0.1, (0.32 * travel) / 3.14);
  return {
    time,
    delay,
    pulseDuration,
    height:
      pulseEnvelope((time - delay) / pulseDuration, 0.35) * amplitude * 0.18,
  };
}
export function surfaceSample(
  x: number,
  y: number,
  phase: number,
  amplitude: number,
  travel = 3,
) {
  const z = Math.sqrt(Math.max(0, 1 - x * x - y * y));
  const length = Math.hypot(x, y, z) || 1;
  const originLength = Math.hypot(-0.3, -0.45, 0.84);
  const dot = (x * -0.3 + y * -0.45 + z * 0.84) / (length * originLength);
  const angle = Math.acos(Math.max(-1, Math.min(1, dot)));
  return {
    z,
    angle,
    front: (phase / 4) * Math.PI,
    ...rippleAtAngle(angle, phase, amplitude, travel),
  };
}
export function impulseSample(time: number, amplitude: number) {
  return { value: pulseEnvelope(time / 0.24, 0.1) * amplitude };
}
