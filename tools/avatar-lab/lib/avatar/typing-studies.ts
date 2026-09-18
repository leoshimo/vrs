import { motionFromPreset, type SavedExpression, type Setup } from './setup';

export const typingStudies: SavedExpression[] = [
  {
    id: 'pressure-dimple',
    name: 'Dimple',
    energy: 0.28,
    spread: 7.5,
    offset: 0.3,
  },
  {
    id: 'pressure-soft-press',
    name: 'Soft press',
    energy: 0.14,
    spread: 1.8,
    offset: 0.36,
  },
  {
    id: 'pressure-edge-press',
    name: 'Edge press',
    energy: 0.26,
    spread: 5,
    offset: 0.8,
  },
].map(({ id, name, energy, spread, offset }) => {
  const motion = motionFromPreset('pressure');
  return {
    id,
    name,
    motions: [
      {
        ...motion,
        settings: {
          ...motion.settings,
          typingEnergy: energy,
          pressureSpread: spread,
          pressureOffset: offset,
        },
      },
    ],
  };
});

export function withTypingStudies(setup: Setup): Setup {
  return {
    ...setup,
    expressions: [
      ...setup.expressions,
      ...typingStudies.filter(
        (study) => !setup.expressions.some((e) => e.id === study.id),
      ),
    ],
  };
}
