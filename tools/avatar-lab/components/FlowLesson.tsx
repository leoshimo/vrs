'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Interactive function plots. */
import { CodeValues, number } from './AnatomyParts';
import { clamp, dragDiagram } from './diagramDrag';
export function FlowLesson({
  x,
  y,
  phase,
  detail,
  onPick,
}: {
  x: number;
  y: number;
  phase: number;
  detail: number;
  onPick: (x: number, y: number) => void;
}) {
  const a = (v: number) => Math.sin(v * 2.8 + phase * 1.4),
    b = (v: number) => Math.cos(v * 3.1 - phase);
  return (
    <>
      <div className="flow-curves">
        {[a, b].map((fn, i) => (
          <figure key={i}>
            <figcaption>
              {i ? 'cos(y × 3.1 − phase)' : 'sin(x × 2.8 + phase × 1.4)'}
            </figcaption>
            <svg
              viewBox="0 0 280 140"
              role="img"
              aria-label={i ? 'Vertical flow slice' : 'Horizontal flow slice'}
              {...dragDiagram((px) => {
                const v = clamp((px - 30) / 220) * 2 - 1;
                onPick(i ? x : v, i ? v : y);
              })}
            >
              <path
                d="M30 65H250M140 15V115"
                stroke="currentColor"
                opacity=".2"
              />
              <path
                d={Array.from(
                  { length: 101 },
                  (_, n) =>
                    `${n ? 'L' : 'M'}${30 + n * 2.2} ${65 - fn(n / 50 - 1) * 45}`,
                ).join(' ')}
                fill="none"
                stroke="var(--lab-accent)"
                strokeWidth="2"
              />
              <circle
                cx={30 + ((i ? y : x) + 1) * 110}
                cy={65 - fn(i ? y : x) * 45}
                r="4"
                fill="var(--lab-accent)"
              />
              <text x="30" y="134">
                −1
              </text>
              <text x="244" y="134">
                1
              </text>
            </svg>
          </figure>
        ))}
      </div>
      <CodeValues
        rows={[
          ['sin(v.x * 2.8 + phase * 1.4)', number(a(x))],
          ['cos(v.y * 3.1 - phase)', number(b(y))],
          [
            'flow = sinX * cosY * .09 * u_idleAmount;',
            number(a(x) * b(y) * 0.09 * detail),
          ],
        ]}
      />
    </>
  );
}
