'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Coordinate diagrams. */
import type { PointerEvent } from 'react';
import {
  sampleDisplacement,
  tutorialDensity,
  type CoordinateOperation,
} from '@/lib/avatar/field-explainer';
export const coordinateNotes = {
  translation: [
    'Translation',
    'Every position receives the same offset. The pattern moves as one sheet beneath the stationary circle.',
  ],
  twist: [
    'Local twist',
    'Nearby positions receive sideways offsets around a moving center.',
  ],
  shear: [
    'Wavy displacement',
    'Horizontal and vertical offsets vary across the picture. Lines bend because neighboring pixels look up different offsets.',
  ],
  vertical: [
    'Vertical displacement',
    'Samples near the center are shifted vertically; the edge barely moves.',
  ],
} as const;
export function CoordinateLab({
  operation = 'translation',
  amount = 1,
  phase = 1,
  x = 0.35,
  y = 0.1,
  onPick,
  sample = (px: number, py: number) => tutorialDensity(px, py, 1, 0, 1, 0.5, 3),
}: {
  sample?: (x: number, y: number) => number;
  operation?: CoordinateOperation;
  amount?: number;
  phase?: number;
  x?: number;
  y?: number;
  onPick?: (x: number, y: number) => void;
}) {
  function pick(event: PointerEvent<SVGSVGElement>) {
    const transform = event.currentTarget.getScreenCTM();
    if (!transform || !onPick) return;
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(
      transform.inverse(),
    );
    onPick((point.x - 105) / 80, (point.y - 100) / 80);
  }
  const interaction = {
    'data-pickable': !!onPick,
    onPointerDown: (event: PointerEvent<SVGSVGElement>) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      pick(event);
    },
    onPointerMove: (event: PointerEvent<SVGSVGElement>) => {
      if (event.buttons) pick(event);
    },
  };
  const displacement = sampleDisplacement(x, y, phase, amount, operation);
  const q = [x - displacement[0], y - displacement[1]];
  const cells = Array.from({ length: 32 * 32 }, (_, i) => ({
    x: (i % 32) / 16 - 1,
    y: Math.floor(i / 32) / 16 - 1,
  }));
  const value = sample(q[0], q[1]);
  return (
    <div className="coordinate-lab">
      <div className="coordinate-panels">
        <figure>
          <figcaption>1 · Source field</figcaption>
          <svg
            viewBox="0 0 210 200"
            {...interaction}
            role="img"
            aria-label="Source field with position and displaced sample"
          >
            {cells.map((p, i) => (
              <rect
                key={i}
                x={25 + (p.x + 1) * 80}
                y={20 + (p.y + 1) * 80}
                width="5.1"
                height="5.1"
                fill="currentColor"
                opacity={sample(p.x, p.y)}
              />
            ))}
            <circle
              cx="105"
              cy="100"
              r="80"
              fill="none"
              stroke="var(--lab-accent)"
              strokeDasharray="3 4"
            />
            <line
              x1={105 + x * 80}
              y1={100 + y * 80}
              x2={105 + q[0] * 80}
              y2={100 + q[1] * 80}
              stroke="var(--lab-accent)"
              strokeWidth="2"
            />
            <circle
              cx={105 + x * 80}
              cy={100 + y * 80}
              r="4"
              fill="white"
              stroke="var(--lab-accent)"
              strokeWidth="2"
            />
            <circle
              cx={105 + q[0] * 80}
              cy={100 + q[1] * 80}
              r="4"
              fill="var(--lab-accent)"
            />
          </svg>
          <small>○ output position · ● source sample</small>
        </figure>
        <figure>
          <figcaption>2 · Sampling offsets</figcaption>
          <svg
            viewBox="0 0 210 200"
            {...interaction}
            role="img"
            aria-label="Offsets at a grid of stationary output pixels"
          >
            <circle
              cx="105"
              cy="100"
              r="80"
              fill="none"
              stroke="currentColor"
              strokeDasharray="3 4"
            />
            {Array.from({ length: 81 }, (_, i) => {
              const px = ((i % 9) - 4) / 4,
                py = (Math.floor(i / 9) - 4) / 4;
              const d = sampleDisplacement(px, py, phase, amount, operation);
              return (
                <g key={i}>
                  <circle
                    cx={105 + px * 70}
                    cy={100 + py * 70}
                    r="1.5"
                    fill="currentColor"
                  />
                  <line
                    x1={105 + px * 70}
                    y1={100 + py * 70}
                    x2={105 + (px - d[0]) * 70}
                    y2={100 + (py - d[1]) * 70}
                    stroke="var(--lab-accent)"
                  />
                  <circle
                    cx={105 + (px - d[0]) * 70}
                    cy={100 + (py - d[1]) * 70}
                    r="1.8"
                    fill="var(--lab-accent)"
                  />
                </g>
              );
            })}
          </svg>
          <small>Lines point to where each pixel looks.</small>
        </figure>
        <figure>
          <figcaption>3 · Output</figcaption>
          <svg
            viewBox="0 0 210 200"
            {...interaction}
            role="img"
            aria-label="Result sampled under an unchanged circular mask"
          >
            {cells
              .filter((p) => Math.hypot(p.x + 0.03, p.y + 0.03) < 1)
              .map((p, i) => {
                const d = sampleDisplacement(
                  p.x,
                  p.y,
                  phase,
                  amount,
                  operation,
                );
                return (
                  <rect
                    key={i}
                    x={25 + (p.x + 1) * 80}
                    y={20 + (p.y + 1) * 80}
                    width="5.1"
                    height="5.1"
                    fill="currentColor"
                    opacity={sample(p.x - d[0], p.y - d[1])}
                  />
                );
              })}
            <circle
              cx="105"
              cy="100"
              r="80"
              fill="none"
              stroke="var(--lab-accent)"
              strokeDasharray="3 4"
            />
            <circle
              cx={105 + x * 80}
              cy={100 + y * 80}
              r="4"
              fill="white"
              stroke="var(--lab-accent)"
              strokeWidth="2"
            />
          </svg>
          <small>The mask stays put. Only its contents change.</small>
        </figure>
      </div>
      <div className="sample-equation">
        <span>
          Position
          <br />
          <b>
            ({x.toFixed(2)}, {y.toFixed(2)})
          </b>
        </span>
        <span>−</span>
        <span>
          Displacement
          <br />
          <b>
            ({displacement[0].toFixed(2)}, {displacement[1].toFixed(2)})
          </b>
        </span>
        <span>=</span>
        <span>
          Sample position
          <br />
          <b>
            ({q[0].toFixed(2)}, {q[1].toFixed(2)})
          </b>
        </span>
        <span>→</span>
        <span>
          Ink coverage
          <br />
          <b>{Math.round(value * 100)}%</b>
        </span>
      </div>
      <p>
        {coordinateNotes[operation][1]} A displacement of 0.25 means one quarter
        of the avatar’s radius.
      </p>
    </div>
  );
}
