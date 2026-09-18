import preset from "./preset.json" with { type: "json" };
import type { AvatarExpression, AvatarSetup, Pattern } from "./expression-model";
import type { SignalConfig } from "./signal-field";

function expression(
  id: string,
  name: string,
  patterns: Pattern[],
  duration = 0.3,
  accent = false,
): AvatarExpression {
  return {
    id,
    name,
    description: "",
    patterns,
    duration,
    color: {
      mode: accent ? "accent" : "none",
      palette: "spectrum",
      strength: 0.3,
      attack: 0.07,
      duration: 0.32,
    },
  };
}

export function assignedSetup(): AvatarSetup {
  return {
    version: 2,
    appearance: structuredClone(preset.appearance) as SignalConfig,
    assignments: {
      idle: "rest",
      typing: "field-nudge",
      working: "working",
      open: "printed-open",
      openAfterIdle: "entrance-print-ripple",
      complete: "sparks",
    },
    expressions: [
      expression("rest", "Lava", [{ kind: "lava", settings: { idleAmount: 1.6 } }]),
      expression(
        "field-nudge",
        "Nudge",
        [{ kind: "nudge", settings: { typingEnergy: 1.04 } }],
        0.24,
      ),
      expression(
        "working",
        "Stir + Ripple",
        [
          { kind: "stir", settings: { loadingStrength: 0.55, loadingRate: 2.45, coupling: 1 } },
          {
            kind: "ripple",
            settings: {
              surfaceAmplitude: 0.28,
              surfaceDuration: 5,
              surfaceWidth: 0.32,
              surfaceOriginX: -0.3,
              surfaceOriginY: -0.45,
            },
          },
        ],
        0.3,
        true,
      ),
      { ...expression("printed-open", "Printed — Subtle", [{ kind: "printed", settings: {} }], 0.15), revealStart: 0.72 },
      expression("bloom", "Bloom", [{ kind: "bloom", settings: { dispatchEnergy: 1.2 } }]),
      expression("entrance-print-ripple", "Printed + Ripple", [
        { kind: "printed", settings: {} },
        {
          kind: "ripple",
          settings: {
            surfaceAmplitude: 0.45,
            surfaceDuration: 0.3,
            surfaceWidth: 0.32,
            surfaceOriginX: -0.7,
            surfaceOriginY: -0.6,
          },
        },
      ]),
      expression("sparks", "Sparks", [
        { kind: "sparks", settings: { dispatchEnergy: 0.8, sparkCount: 11 } },
      ]),
    ],
  };
}
