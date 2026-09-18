import { motionFromPreset, type Setup, type SavedExpression } from './setup';

const studies: SavedExpression[] = [
  ...[
    'swirl',
    'orbit',
    'nudge-sparks',
    'entrance-print',
    'entrance-print-ripple',
    'entrance-fill-ripple',
  ].map((id) => ({
    id,
    name: {
      swirl: 'Swirl',
      orbit: 'Orbit',
      'nudge-sparks': 'Nudge + Sparks',
      'entrance-print': 'Printed',
      'entrance-print-ripple': 'Printed + Ripple',
      'entrance-fill-ripple': 'Fill + Ripple',
    }[id]!,
    motions: [motionFromPreset(id)],
  })),
  ...(
    [
      {
        id: 'opal-nudge',
        name: 'Opal nudge',
        preset: 'field-nudge',
        color: 'opal',
      },
      {
        id: 'ember-sparks',
        name: 'Ember sparks',
        preset: 'sparks',
        color: 'ember',
      },
    ] as const
  ).map(({ id, name, preset, color }) => {
    const motion = motionFromPreset(preset);
    return {
      id,
      name,
      motions: [
        {
          ...motion,
          settings: {
            ...motion.settings,
            activityColor: color,
            colorStrength: 0.9,
          },
        },
      ],
    };
  }),
];

export function withLabStudies(setup: Setup): Setup {
  return {
    ...setup,
    events: {
      ...setup.events,
      flourish:
        setup.events.flourish === 'surface-submit'
          ? 'entrance-print-ripple'
          : setup.events.flourish,
    },
    appearance: { activityColor: 'opal', ...setup.appearance },
    expressions: [
      ...setup.expressions.map((e) =>
        e.name === 'Fill wake' ? { ...e, name: 'Fill' } : e,
      ),
      ...studies.filter((s) => !setup.expressions.some((e) => e.id === s.id)),
    ],
  };
}

// Sampling happens once per gesture. A fixed seed makes an experiment repeatable.
export function variationPicker(seed = 1) {
  let state = seed >>> 0;
  return (choices: readonly (readonly [string, number])[]) => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    let position =
      (state / 0x100000000) *
      choices.reduce((sum, [, weight]) => sum + weight, 0);
    for (const [id, weight] of choices) {
      position -= weight;
      if (position < 0) return id;
    }
    return choices.at(-1)![0];
  };
}

export const variations = {
  flourish: [
    ['entrance-print', 2],
    ['entrance-print-ripple', 1],
    ['entrance-fill-ripple', 1],
  ],
  typing: [
    ['field-nudge', 9],
    ['nudge-sparks', 1],
  ],
} as const;
