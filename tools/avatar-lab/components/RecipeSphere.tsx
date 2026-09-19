'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Interactive coordinate plot. */
import { useMemo } from 'react';
import { ExpressionPreview } from './ExpressionPreview';
import { ExpressionPlayer } from '../../../vrsjmp/src/avatar/expression-player';
import {
  assignmentNames,
  type AvatarSetup,
} from '../../../vrsjmp/src/avatar/expression-model';
import { dragDiagram } from './diagramDrag';
import type { SignalConfig } from '@/lib/avatar/signal-field';
export function RecipeSphere({
  config,
  theme,
  label,
  x,
  y,
  onPick,
}: {
  config: SignalConfig;
  theme: 'light' | 'dark';
  label: string;
  x: number;
  y: number;
  onPick: (x: number, y: number) => void;
}) {
  const sample = useMemo(() => {
    const setup: AvatarSetup = {
      appearance: config,
      assignments: Object.fromEntries(
        assignmentNames.map((a) => [a, null]),
      ) as AvatarSetup['assignments'],
    };
    if (config.surfaceWave) {
      setup.assignments.submit = {
        id: 'sample-ripple',
        name: 'Ripple',
        description: '',
        duration: 0.3,
        patterns: [
          {
            kind: 'ripple',
            settings: {
              strength: config.surfaceWave,
              travel: config.surfaceDuration ?? 3,
              width: config.surfaceWidth ?? 0.32,
              originX: config.surfaceOriginX ?? -0.3,
              originY: config.surfaceOriginY ?? -0.45,
            },
          },
        ],
        color: {
          mode: 'none',
          palette: 'mono',
          strength: 0,
          attack: 0.07,
          duration: 0.3,
        },
      };
    }
    const player = new ExpressionPlayer(setup);
    if (config.surfaceWave) {
      player.trigger('submit');
      const seconds =
        ((config.surfacePhase ?? 0) / 4) * (config.surfaceDuration ?? 3);
      for (let elapsed = 0; elapsed < seconds; elapsed += 1 / 120)
        player.advance(Math.min(1 / 120, seconds - elapsed));
    }
    const phase = config.inspectPhase ?? 0;
    player.setPose(
      phase * (config.lightRotationRate ?? 1),
      phase * (config.flowRate ?? 1),
    );
    return { setup, player };
  }, [config]);
  return (
    <figure className="recipe-sphere">
      <figcaption>{label}</figcaption>
      <div data-appearance={theme}>
        <ExpressionPreview
          setup={sample.setup}
          engine={sample.player}
          dark={theme === 'dark'}
          size={96}
          zoom={3}
          label={label}
        />
        <svg
          viewBox="0 0 166 166"
          role="img"
          aria-label={`${label}: draggable sample point`}
          {...dragDiagram((px, py) =>
            onPick((px - 83) / 49.14, (py - 83) / 49.14),
          )}
        >
          <circle
            cx={83 + x * 49.14}
            cy={83 + y * 49.14}
            r="4"
            fill="var(--lab-accent)"
            stroke="white"
            strokeWidth="1.5"
          />
        </svg>
      </div>
      <small>96 px · 3×</small>
    </figure>
  );
}
