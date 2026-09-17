'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- SVG mathematical diagrams. */
import { useId, type PointerEvent } from 'react';
import { dragDiagram, clamp, radialPoint } from './diagramDrag';
import { ShaderCode } from './ShaderCode';
import {
  bayerThreshold,
  impulseSample,
  sampleDisplacement,
  surfaceSample,
  toneBand,
  tutorialDensity,
  type CoordinateOperation,
} from '@/lib/avatar/field-explainer';
export const number = (v: number) =>
  Math.abs(v) < 0.0005 ? '0.000' : v.toFixed(3);
export function CodeValues({ rows }: { rows: [string, string][] }) {
  return (
    <div className="code-values">
      {rows.map(([code, value], i) => (
        <div key={i}>
          <ShaderCode code={code} />
          <output>{value}</output>
        </div>
      ))}
    </div>
  );
}
export function ScalarField({
  kind,
  x = 0.35,
  y = 0,
  volume = 1,
  angle = 0,
  strength = 1,
  fill = 0.5,
  onPick,
}: {
  kind: 'mask' | 'dome' | 'gradient' | 'combined';
  x?: number;
  y?: number;
  volume?: number;
  angle?: number;
  strength?: number;
  fill?: number;
  onPick?: (x: number, y: number) => void;
}) {
  const id = useId();
  function pick(event: PointerEvent<SVGSVGElement>) {
    if (!onPick) return;
    const svg = event.currentTarget,
      ctm = svg.getScreenCTM();
    if (!ctm) return;
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(
      ctm.inverse(),
    );
    onPick(
      Math.max(-1.1, Math.min(1.1, (point.x - 110) / 100)),
      Math.max(-1.1, Math.min(1.1, (point.y - 110) / 100)),
    );
  }
  return (
    <svg
      className="scalar-field"
      data-pickable={!!onPick}
      onPointerDown={
        onPick
          ? (event) => {
              event.currentTarget.setPointerCapture(event.pointerId);
              pick(event);
            }
          : undefined
      }
      onPointerMove={
        onPick
          ? (event) => {
              if (event.buttons) pick(event);
            }
          : undefined
      }
      viewBox="0 0 220 220"
      role="img"
      aria-label={`${kind} at every position; marker is the inspected point`}
    >
      <defs>
        <pattern id={id} width="10" height="10" patternUnits="userSpaceOnUse">
          <path
            d="M10 0H0V10"
            stroke="currentColor"
            strokeWidth=".3"
            opacity=".2"
            fill="none"
          />
        </pattern>
      </defs>
      <rect x="10" y="10" width="200" height="200" fill={`url(#${id})`} />
      {Array.from({ length: 1600 }, (_, i) => {
        const px = ((i % 40) + 0.5) / 20 - 1,
          py = (Math.floor(i / 40) + 0.5) / 20 - 1;
        if (px * px + py * py > 1) return null;
        const value =
          kind === 'mask'
            ? 1
            : kind === 'dome'
              ? Math.sqrt(Math.max(0, 1 - px * px - py * py))
              : kind === 'gradient'
                ? 0.5 +
                  0.5 *
                    (px * Math.cos((angle * Math.PI) / 180) +
                      py * Math.sin((angle * Math.PI) / 180)) *
                    strength
                : tutorialDensity(px, py, volume, angle, strength, fill, 3);
        return (
          <rect
            key={i}
            x={10 + (i % 40) * 5}
            y={10 + Math.floor(i / 40) * 5}
            width="5.1"
            height="5.1"
            fill="currentColor"
            opacity={Math.max(0, Math.min(1, value))}
          />
        );
      })}
      <circle
        cx="110"
        cy="110"
        r="100"
        fill="none"
        stroke="var(--lab-accent)"
        strokeDasharray="3 3"
      />
      <circle
        cx={110 + x * 100}
        cy={110 + y * 100}
        r="4"
        fill="var(--lab-accent)"
        stroke="white"
        strokeWidth="1.5"
      />
    </svg>
  );
}
export function HemisphereSection({
  x,
  y = 0,
  onPick,
}: {
  x: number;
  y?: number;
  onPick?: (x: number, y: number) => void;
}) {
  const r = Math.min(1, Math.hypot(x, y)),
    z = Math.sqrt(1 - r * r);
  const cx = 175,
    baseline = 170,
    scale = 135,
    px = cx + r * scale,
    py = baseline - z * scale;
  return (
    <figure className="dome-triangle">
      <svg
        viewBox="0 0 380 215"
        {...dragDiagram(
          onPick
            ? (px, py) => {
                const angle = Math.atan2(
                  Math.max(0, baseline - py),
                  Math.max(0, px - cx),
                );
                onPick(...radialPoint(x, y, Math.cos(angle)));
              }
            : undefined,
        )}
        role="img"
        aria-label={`Cross-section of a unit sphere. A right triangle has horizontal distance ${r.toFixed(2)}, height ${z.toFixed(2)}, and radius one.`}
      >
        <path
          d="M40 170 A135 135 0 0 1 310 170"
          fill="none"
          stroke="currentColor"
          opacity=".3"
          strokeWidth="2"
        />
        <path d="M30 170 H345" stroke="currentColor" opacity=".25" />
        <path
          d={`M${cx} ${baseline} L${px} ${baseline} L${px} ${py} Z`}
          fill="var(--lab-accent)"
          fillOpacity=".10"
          stroke="var(--lab-accent)"
          strokeWidth="2"
        />
        {r > 0.07 && z > 0.07 && (
          <path
            d={`M${px - 9} ${baseline}v-9h9`}
            fill="none"
            stroke="currentColor"
            opacity=".5"
          />
        )}
        <circle cx={px} cy={py} r="4" fill="var(--lab-accent)" />
        <text x={cx + (r * scale) / 2} y="194" textAnchor="middle">
          r = {r.toFixed(2)}
        </text>
        <text x={px + 10} y={(py + baseline) / 2}>
          z = {z.toFixed(2)}
        </text>
        <text
          x={(cx + px) / 2 - 18}
          y={(baseline + py) / 2 - 12}
          textAnchor="middle"
        >
          radius = 1
        </text>
        <text x="38" y="28">
          Side view
        </text>
      </svg>
      <figcaption>
        The radius is the triangle’s longest side. Moving toward the edge
        increases r and reduces height z.
      </figcaption>
    </figure>
  );
}
export function CompositionLesson({
  volume,
  strength,
  angle,
  fill,
  x,
  y,
}: {
  volume: number;
  strength: number;
  angle: number;
  fill: number;
  x: number;
  y: number;
}) {
  const dome = Math.sqrt(Math.max(0, 1 - x * x - y * y));
  const projection =
    x * Math.cos((angle * Math.PI) / 180) +
    y * Math.sin((angle * Math.PI) / 180);
  const raw = 0.4 + 0.25 * dome * volume + 0.34 * projection * strength;
  return (
    <>
      <div className="field-sum">
        <figure>
          <figcaption>dome</figcaption>
          <ScalarField kind="dome" x={x} y={y} />
          <small>× .25 × u_volume</small>
        </figure>
        <b>+</b>
        <figure>
          <figcaption>directional gradient</figcaption>
          <ScalarField
            kind="gradient"
            angle={angle}
            strength={strength}
            x={x}
            y={y}
          />
          <small>signed · × .34</small>
        </figure>
        <b>+</b>
        <figure className="base-term">
          <figcaption>base</figcaption>
          <strong>.40</strong>
        </figure>
        <b>=</b>
        <figure>
          <figcaption>combined tone</figcaption>
          <ScalarField
            kind="combined"
            volume={volume}
            strength={strength}
            angle={angle}
            fill={0.5}
            x={x}
            y={y}
          />
          <small>before fill remapping</small>
        </figure>
      </div>
      <CodeValues
        rows={[
          ['.25 * dome * u_volume', number(0.25 * dome * volume)],
          [
            '.34 * dot(v, light) * u_lightStrength',
            number(0.34 * projection * strength),
          ],
          ['tone = .40 + domeTerm + gradientTerm;', number(raw)],
          [
            'tone = clamp((tone - .5) * u_contrast + .5, .015, .985);',
            number(Math.max(0.015, Math.min(0.985, raw))),
          ],
          [
            fill < 0.5
              ? 'tone *= u_fill * 2.;'
              : 'tone = mix(tone, 1., (u_fill - .5) * 2.);',
            number(tutorialDensity(x, y, volume, angle, strength, fill, 3)),
          ],
        ]}
      />
    </>
  );
}
export function MatrixConstruction() {
  return (
    <div className="matrix-construction">
      {[2, 4].map((n) => (
        <figure key={n}>
          <figcaption>
            {n} × {n} ranks
          </figcaption>
          <div
            className="rank-matrix"
            style={{ gridTemplateColumns: `repeat(${n},1fr)` }}
          >
            {Array.from({ length: n * n }, (_, i) => (
              <span key={i}>
                {Math.round(
                  bayerThreshold(i % n, Math.floor(i / n), n) * n * n - 0.5,
                )}
              </span>
            ))}
          </div>
        </figure>
      ))}
      <div>
        <code>B₂ = [[0, 2], [3, 1]]</code>
        <p>
          For the next size, repeat the current tile four times. Multiply its
          ranks by 4, then add 0, 2, 3 or 1 to each quadrant.
        </p>
        <code>threshold = (rank + .5) / (N * N)</code>
        <p>
          Low ranks print first. Their spacing produces the crosses and
          checkerboards.
        </p>
      </div>
    </div>
  );
}
export function BandLesson({
  count,
  density,
  onDensity,
}: {
  onDensity?: (v: number) => void;
  count: number;
  density: number;
}) {
  const level = toneBand(density, count);
  return (
    <>
      <div className="band-comparison">
        {['Continuous tone', 'Quantized level', 'Final coverage'].map(
          (label, k) => (
            <figure key={label}>
              <figcaption>{label}</figcaption>
              <div
                className="band-ramp"
                data-draggable={!!onDensity}
                onPointerDown={(e) => {
                  e.currentTarget.setPointerCapture(e.pointerId);
                  const b = e.currentTarget.getBoundingClientRect();
                  onDensity?.(clamp((e.clientX - b.left) / b.width));
                }}
                onPointerMove={(e) => {
                  if (e.buttons) {
                    const b = e.currentTarget.getBoundingClientRect();
                    onDensity?.(clamp((e.clientX - b.left) / b.width));
                  }
                }}
              >
                <span
                  className="band-marker"
                  style={{ left: `${density * 100}%` }}
                />
                {Array.from({ length: 96 }, (_, i) => {
                  const d = i / 95,
                    l = toneBand(d, count);
                  return (
                    <i
                      key={i}
                      style={{
                        background: 'currentColor',
                        opacity: k === 0 ? d : k === 1 ? l : (d + l) / 2,
                      }}
                    />
                  );
                })}
              </div>
            </figure>
          ),
        )}
      </div>
      <CodeValues
        rows={[
          ['tone', number(density)],
          [
            'level = floor(clamp(tone, 0., .999) * N) / (N - 1.);',
            number(level),
          ],
          [
            'orbInk = mix(paper, orbInk, .30 + .70 * level);',
            `${number(0.3 + 0.7 * level)} pigment`,
          ],
          ['tone = mix(tone, level, .5);', number((density + level) / 2)],
        ]}
      />
    </>
  );
}
export function OperationValues({
  operation,
  phase,
  amount,
  x = 0.35,
  y = 0.1,
}: {
  operation: CoordinateOperation;
  phase: number;
  amount: number;
  x?: number;
  y?: number;
}) {
  const d = sampleDisplacement(x, y, phase, amount, operation);
  const q = [x - 0.2 * Math.sin(phase), y - 0.2 * Math.cos(phase * 0.83)];
  const rows: [string, string][] =
    operation === 'translation'
      ? [
          ['sin(phase * .9)', number(Math.sin(phase * 0.9))],
          ['cos(phase * .67)', number(Math.cos(phase * 0.67))],
          [
            'displacement = amount * .24 * vec2(sin(phase * .9), cos(phase * .67));',
            `(${number(d[0])}, ${number(d[1])})`,
          ],
        ]
      : operation === 'twist'
        ? [
            [
              'center = .2 * vec2(sin(phase), cos(phase * .83));',
              `(${number(x - q[0])}, ${number(y - q[1])})`,
            ],
            ['q = v - center;', `(${number(q[0])}, ${number(q[1])})`],
            [
              'tangent = vec2(-q.y, q.x);',
              `(${number(-q[1])}, ${number(q[0])})`,
            ],
            [
              'falloff = exp(-dot(q, q) * 1.8);',
              number(Math.exp(-(q[0] ** 2 + q[1] ** 2) * 1.8)),
            ],
            [
              'displacement = amount * tangent * falloff * sin(phase * 1.6) * .65;',
              `(${number(d[0])}, ${number(d[1])})`,
            ],
          ]
        : operation === 'shear'
          ? [
              [
                'xOffset = amount * .19 * sin(v.y * 4. + phase * 2.7);',
                number(d[0]),
              ],
              [
                'yOffset = amount * .19 * cos(v.x * 3.4 - phase * 2.1);',
                number(d[1]),
              ],
            ]
          : [
              [
                'yOffset = amount * .30 * (1. - smoothstep(.15, 1.1, length(v)));',
                number(d[1]),
              ],
            ];
  return (
    <CodeValues
      rows={[
        ['v', `(${number(x)}, ${number(y)})`],
        ...rows,
        [
          'sampleV = v - displacement;',
          `(${number(x - d[0])}, ${number(y - d[1])})`,
        ],
      ]}
    />
  );
}
export function SurfaceWaveLesson({
  phase,
  amplitude,
  part,
  x = 0.35,
  y = 0.1,
  onPick,
  onPhase,
}: {
  phase: number;
  amplitude: number;
  part: number;
  x?: number;
  y?: number;
  onPick?: (x: number, y: number) => void;
  onPhase?: (v: number) => void;
}) {
  const sample = surfaceSample(x, y, phase, amplitude);
  const path = (withWave: boolean) =>
    Array.from({ length: 121 }, (_, i) => {
      const angle = -Math.PI / 2 + (i * Math.PI) / 120,
        x = Math.sin(angle),
        z = Math.cos(angle);
      const h = withWave ? surfaceSample(x, 0, phase, amplitude).height : 0;
      return `${i ? 'L' : 'M'}${165 + x * (1 + h) * 125} ${155 - z * (1 + h) * 120}`;
    }).join(' ');
  return (
    <>
      <div className="wave-sections">
        <figure>
          <figcaption>Cross-section through the sphere</figcaption>
          <svg
            viewBox="0 0 330 185"
            role="img"
            aria-label="A wave raises and lowers the sphere surface"
            {...dragDiagram(
              onPick
                ? (px, py) => {
                    const angle = Math.atan2(
                      clamp((px - 165) / 125, -1, 1),
                      Math.max(0.001, (155 - py) / 120),
                    );
                    onPick(Math.sin(angle), 0);
                  }
                : undefined,
            )}
          >
            <path
              d={path(false)}
              fill="none"
              stroke="currentColor"
              opacity=".3"
              strokeDasharray="3 3"
            />
            <path
              d={path(true)}
              fill="none"
              stroke="var(--lab-accent)"
              strokeWidth="2"
            />
            <circle
              cx={165 + x * (1 + sample.height) * 125}
              cy={
                155 -
                Math.sqrt(Math.max(0, 1 - x * x)) * (1 + sample.height) * 120
              }
              r="4"
              fill="var(--lab-accent)"
              stroke="white"
            />
            <path d="M20 155H310" stroke="currentColor" opacity=".2" />
            <text x="28" y="178">
              x: left → right · z: height above the circle
            </text>
          </svg>
        </figure>
        <figure>
          <figcaption>Height along the surface</figcaption>
          <svg
            viewBox="0 0 330 185"
            role="img"
            aria-label="The localized ridge moves with its wave front"
            {...dragDiagram(
              onPhase
                ? (px) => onPhase((clamp((px - 20) / 290) * Math.PI) / 0.82)
                : undefined,
            )}
          >
            <path d="M20 95H310" stroke="currentColor" opacity=".3" />
            <path
              d={Array.from({ length: 151 }, (_, i) => {
                const a = (i / 150) * Math.PI,
                  d = a - sample.front,
                  e = Math.exp(-((d / 0.32) ** 2));
                const h =
                  Math.sin(d * 12) *
                  e *
                  amplitude *
                  0.16 *
                  (phase < 0.22
                    ? (phase / 0.22) ** 2 * (3 - (2 * phase) / 0.22)
                    : 1) *
                  (phase > 2.8
                    ? 1 -
                      Math.min(1, phase - 2.8) ** 2 *
                        (3 - 2 * Math.min(1, phase - 2.8))
                    : 1);
                return `${i ? 'L' : 'M'}${20 + (i / 150) * 290} ${95 - h * 250}`;
              }).join(' ')}
              stroke="var(--lab-accent)"
              fill="none"
              strokeWidth="2"
            />
            <line
              x1={20 + (sample.front / Math.PI) * 290}
              x2={20 + (sample.front / Math.PI) * 290}
              y1="25"
              y2="153"
              stroke="currentColor"
              strokeDasharray="3 3"
            />
            <text x="25" y="178">
              angular distance from origin: 0 → π
            </text>
          </svg>
        </figure>
      </div>
      <CodeValues
        rows={
          part === 0
            ? [
                ['v', `(${number(x)}, ${number(y)})`],
                [
                  'point = normalize(vec3(v, sqrt(max(0., 1. - dot(v, v)))));',
                  `z = ${number(sample.z)}`,
                ],
                [
                  'origin = normalize(vec3(u_surfaceOrigin, .84));',
                  'u_surfaceOrigin = (-.3, -.45)',
                ],
                [
                  'angle = acos(clamp(dot(point, origin), -1., 1.));',
                  `${number(sample.angle)} rad`,
                ],
              ]
            : part === 1
              ? [
                  ['width; wavelength; damping;', '.32 rad; π/6 rad; 0'],
                  ['front = phase * .82;', `${number(sample.front)} rad`],
                  ['distance = angle - front;', number(sample.distance)],
                  [
                    'envelope = exp(-pow(distance / width, 2.)) * fadeIn * fadeOut;',
                    number(sample.envelope),
                  ],
                  [
                    'height = sin(distance * 2π / wavelength) * envelope * exp(-damping * front) * amplitude * .16;',
                    number(sample.height),
                  ],
                ]
              : [
                  ['height', number(sample.height)],
                  [
                    'posed = v / (1. + height);',
                    `(${number(x / (1 + sample.height))}, ${number(y / (1 + sample.height))})`,
                  ],
                  [
                    'displacement += v * height;',
                    `(${number(x * sample.height)}, ${number(y * sample.height)})`,
                  ],
                  [
                    'flow += height * 1.4;',
                    `${number(sample.height * 1.4)} coverage`,
                  ],
                ]
        }
      />
      <p>
        {part === 0
          ? 'Lift each 2D position onto a unit hemisphere. Both vectors have length 1, so their dot product gives the cosine of the angle between them. acos turns that into distance measured around the sphere.'
          : part === 1
            ? 'The front travels outward. A sine makes crests and troughs; a Gaussian envelope confines them to a narrow band. The start and end fades bring the height back to zero.'
            : 'The same height feeds three places: the boundary test, the sample coordinates, and the ink coverage. That shared value makes the silhouette and printed interior react together.'}
      </p>
    </>
  );
}
export function ImpulseGraph({
  time,
  amplitude,
  onTime,
}: {
  time: number;
  amplitude: number;
  onTime?: (n: number) => void;
}) {
  const s = impulseSample(time, amplitude);
  return (
    <>
      <svg
        className="impulse-graph"
        {...dragDiagram(
          onTime ? (px) => onTime(clamp((px - 20) / 400)) : undefined,
        )}
        viewBox="0 0 440 160"
        role="img"
        aria-label="Spring displacement rises rapidly after a kick, then decays to rest"
      >
        <path d="M20 125H420" stroke="currentColor" opacity=".3" />
        <path
          d={Array.from(
            { length: 121 },
            (_, i) =>
              `${i ? 'L' : 'M'}${20 + (i / 120) * 400} ${125 - impulseSample(i / 120, amplitude).value * 100}`,
          ).join(' ')}
          stroke="var(--lab-accent)"
          strokeWidth="2"
          fill="none"
        />
        <line
          x1={20 + time * 400}
          x2={20 + time * 400}
          y1="15"
          y2="125"
          stroke="currentColor"
          strokeDasharray="3 3"
        />
        <circle
          cx={20 + time * 400}
          cy={125 - s.value * 100}
          r="4"
          fill="var(--lab-accent)"
        />
        <text x="20" y="150">
          0 s · impulse
        </text>
        <text x="350" y="150">
          1 s · settled
        </text>
      </svg>
      <CodeValues
        rows={[
          ['initialVelocity = 16 * amplitude;', number(s.initialVelocity)],
          ['value = initialVelocity * t * exp(-13 * t);', number(s.value)],
          [
            'velocity = initialVelocity * (1 - 13 * t) * exp(-13 * t);',
            number(s.velocity),
          ],
          [
            'light += u_poke * .65 * u_expression;',
            `x + ${number(s.value * 0.65 * 1.25)}`,
          ],
        ]}
      />
    </>
  );
}
export function ClockLesson({
  phase,
  onPhase,
  x = 0.35,
  y = 0.1,
  detail = 1.9,
  lightOffset = 0,
}: {
  phase: number;
  onPhase?: (v: number) => void;
  x?: number;
  y?: number;
  detail?: number;
  lightOffset?: number;
}) {
  const curves: [string, (t: number) => number][] = [
    [
      'Gradient direction',
      (t) => (t - 0.7 + 0.24 * Math.sin(t * 1.7) + lightOffset + 1) / 10,
    ],
    [
      `Flow at (${number(x)}, ${number(y)})`,
      (t) =>
        0.5 +
        Math.sin(x * 2.8 + t * 1.4) * Math.cos(y * 3.1 - t) * 0.09 * detail,
    ],
    [
      'Palette coordinate',
      (t) => 0.5 + 0.5 * Math.sin(x * 1.8 + y * 0.9 + t * 0.7),
    ],
  ];
  return (
    <div className="clock-curves">
      {curves.map(([label, fn]) => (
        <figure key={label}>
          <figcaption>{label}</figcaption>
          <svg
            viewBox="0 0 440 115"
            {...dragDiagram(
              onPhase ? (px) => onPhase(clamp((px - 20) / 400) * 8) : undefined,
            )}
            role="img"
            aria-label={`${label} over phase zero to eight`}
          >
            <path d="M20 90H420" stroke="currentColor" opacity=".2" />
            <path
              d={Array.from(
                { length: 161 },
                (_, i) =>
                  `${i ? 'L' : 'M'}${20 + (i / 160) * 400} ${90 - fn(i / 20) * 70}`,
              ).join(' ')}
              stroke="var(--lab-accent)"
              strokeWidth="1.5"
              fill="none"
            />
            <line
              x1={20 + (phase / 8) * 400}
              x2={20 + (phase / 8) * 400}
              y1="10"
              y2="90"
              stroke="currentColor"
              strokeDasharray="3 3"
            />
            <circle
              cx={20 + (phase / 8) * 400}
              cy={90 - fn(phase) * 70}
              r="3"
              fill="var(--lab-accent)"
            />
            <text x="20" y="108">
              phase 0
            </text>
            <text x="378" y="108">
              phase 8
            </text>
          </svg>
        </figure>
      ))}
    </div>
  );
}
