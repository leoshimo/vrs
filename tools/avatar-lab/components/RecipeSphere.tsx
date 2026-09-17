'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Interactive coordinate plot. */
import { AvatarViewer } from './AvatarViewer';
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
  return (
    <figure className="recipe-sphere">
      <figcaption>{label}</figcaption>
      <div data-appearance={theme}>
        <AvatarViewer
          config={config}
          theme={theme}
          size={96}
          displaySize={42}
          zoom={3}
          animate={false}
          active
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
