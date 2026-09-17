import type { SignalColor } from "./signal-field";
export const paletteOptions: readonly (readonly [SignalColor, string])[] = [
  ["mono", "Monochrome"],
  ["field", "Instrument"],
  ["opal", "Opal"],
  ["spectrum", "Spectrum"],
  ["aurora", "Aurora"],
  ["mist", "Mist"],
  ["dusk", "Dusk"],
  ["slate", "Slate"],
  ["moss", "Moss"],
  ["plum", "Plum"],
  ["ochre", "Ochre"],
];
// A quiet UI accent drawn from each avatar palette; it does not animate the rows.
export const paletteAccents: Partial<Record<SignalColor, string>> = {
  mono: "#90958e",
  field: "#8caa9b",
  opal: "#b3a6bd",
  spectrum: "#b79b98",
  aurora: "#8baaa8",
  mist: "#a2adb0",
  dusk: "#b0a3ad",
  slate: "#8498aa",
  moss: "#8da18e",
  plum: "#aa90a1",
  ochre: "#b3a17d",
};
