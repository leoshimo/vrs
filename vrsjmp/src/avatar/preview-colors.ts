import { paletteAccents } from "./avatar-palettes";
import type { ExpressionPlayer } from "./expression-player";

export function neutralInk(dark: boolean): [number, number, number] {
  const value = dark ? 0.9 : 0.25;
  return [value, value, value];
}

export function applyPlaceholderAccent(input: HTMLElement, player: ExpressionPlayer) {
  const colors = player.colorSignals();
  const dominant = colors.reduce<(typeof colors)[number] | undefined>(
    (best, next) => (!best || next.strength > best.strength ? next : best),
    undefined,
  );
  const strength = Math.min(
    0.8,
    colors.reduce((sum, c) => sum + c.strength, 0),
  );
  const ink = dominant ? (paletteAccents[dominant.palette] ?? "var(--ink)") : "var(--ink)";
  const weight = `${strength < 0.0001 ? 0 : Math.round(strength * 10000) / 100}%`;
  if (input.style.getPropertyValue("--placeholder-accent") !== ink)
    input.style.setProperty("--placeholder-accent", ink);
  if (input.style.getPropertyValue("--placeholder-accent-weight") !== weight)
    input.style.setProperty("--placeholder-accent-weight", weight);
}
