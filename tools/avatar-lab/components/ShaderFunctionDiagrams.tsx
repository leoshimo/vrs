'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- These SVGs are mathematical diagrams. */
import type { ReactNode, PointerEvent } from 'react';
import { RangeControl } from './AvatarControls';
import { dragDiagram, radialPoint } from './diagramDrag';

const f = (v: number) => (Math.abs(v) < 0.0005 ? '0.000' : v.toFixed(3));
const clamp = (v: number) => Math.max(0, Math.min(1, v));
function FunctionCard({
  name,
  input,
  output,
  children,
}: {
  name: string;
  input: string;
  output: string;
  children: ReactNode;
}) {
  return (
    <section
      className="shader-function-card"
      aria-label={`${name} input and output`}
    >
      <h3>
        <code>{name}</code>
      </h3>
      <div className="function-io">
        <div>
          <span>Input</span>
          <code>{input}</code>
        </div>
        <div>
          <span>Output</span>
          <output>{output}</output>
        </div>
      </div>
      {children}
    </section>
  );
}
export function Curve({
  fn,
  value,
  min = 0,
  max = 1,
  inputLabel,
  outputLabel,
  onChange,
  linear = false,
}: {
  fn: (n: number) => number;
  value: number;
  min?: number;
  max?: number;
  inputLabel: string;
  outputLabel: string;
  onChange?: (n: number) => void;
  linear?: boolean;
}) {
  const px = (v: number) => 34 + 220 * clamp((v - min) / (max - min));
  const py = (v: number) => 130 - 100 * clamp(v);
  return (
    <svg
      className="function-plot"
      {...dragDiagram(
        onChange
          ? (px) => onChange(min + clamp((px - 34) / 220) * (max - min))
          : undefined,
      )}
      viewBox="0 0 280 170"
      role="img"
      aria-label={`${outputLabel} as a function of ${inputLabel}; selected input ${f(value)}, output ${f(fn(value))}`}
    >
      <path
        d="M34 22V130H260"
        stroke="currentColor"
        opacity=".25"
        fill="none"
      />
      <text x="24" y="30" textAnchor="end">
        1
      </text>
      <text x="24" y="134" textAnchor="end">
        0
      </text>
      {linear && (
        <path
          d="M34 130L254 30"
          fill="none"
          stroke="currentColor"
          strokeDasharray="4 4"
          opacity=".35"
        />
      )}
      <path
        d={Array.from({ length: 81 }, (_, i) => {
          const n = min + ((max - min) * i) / 80;
          return `${i ? 'L' : 'M'}${px(n)} ${py(fn(n))}`;
        }).join(' ')}
        fill="none"
        stroke="var(--lab-accent)"
        strokeWidth="2.5"
      />
      <path
        d={`M${px(value)} 130V${py(fn(value))}H34`}
        stroke="currentColor"
        opacity=".45"
        strokeDasharray="3 4"
        fill="none"
      />
      <circle
        cx={px(value)}
        cy={py(fn(value))}
        r="4.5"
        fill="var(--lab-accent)"
      />
      <text x="34" y="147">
        {min}
      </text>
      <text x="254" y="147" textAnchor="end">
        {max}
      </text>
      <text x="144" y="165" textAnchor="middle">
        {inputLabel}
      </text>
    </svg>
  );
}
export function ShaderFunctionDiagrams({
  step,
  x,
  y,
  angle,
  onPick,
  part,
}: {
  part?: 'length' | 'edge';
  step: number;
  x: number;
  y: number;
  angle: number;
  onPick: (x: number, y: number) => void;
}) {
  function pickVector(event: PointerEvent<SVGSVGElement>) {
    const transform = event.currentTarget.getScreenCTM();
    if (!transform) return;
    const p = new DOMPoint(event.clientX, event.clientY).matrixTransform(
      transform.inverse(),
    );
    onPick((p.x - 140) / 60, (p.y - 85) / 60);
  }
  const r = Math.hypot(x, y),
    squared = x * x + y * y;
  const distance = r - 1;
  const smooth = (d: number) => {
    const t = clamp((d + 0.025) / 0.05);
    return t * t * (3 - 2 * t);
  };
  const dx = Math.cos((angle * Math.PI) / 180),
    dy = Math.sin((angle * Math.PI) / 180);
  if (step > 2) return null;
  return (
    <div className="shader-function-grid">
      {step === 0 && (
        <>
          {part !== 'edge' && (
            <FunctionCard
              name="length(v)"
              input={`v = (${f(x)}, ${f(y)})`}
              output={`length = ${f(r)}`}
            >
              <svg
                className="function-plot"
                data-pickable="true"
                onPointerDown={(event) => {
                  event.currentTarget.setPointerCapture(event.pointerId);
                  pickVector(event);
                }}
                onPointerMove={(event) => {
                  if (event.buttons) pickVector(event);
                }}
                viewBox="0 0 280 170"
                role="img"
                aria-label="The length of a vector is the hypotenuse of its x and y components"
              >
                <path
                  d="M35 85H245M140 12V158"
                  stroke="currentColor"
                  opacity=".18"
                />
                <circle
                  cx="140"
                  cy="85"
                  r="60"
                  stroke="currentColor"
                  opacity=".2"
                  fill="none"
                />
                <path
                  d={`M140 85H${140 + x * 60}V${85 + y * 60}`}
                  stroke="currentColor"
                  strokeDasharray="3 4"
                  fill="none"
                />
                <path
                  d={`M140 85L${140 + x * 60} ${85 + y * 60}`}
                  stroke="var(--lab-accent)"
                  strokeWidth="3"
                />
                <circle
                  cx={140 + x * 60}
                  cy={85 + y * 60}
                  r="4"
                  fill="var(--lab-accent)"
                />
                <text x="244" y="80">
                  x
                </text>
                <text x="147" y="18">
                  y
                </text>
                <text x="12" y="163">
                  √(x² + y²) = {f(r)}
                </text>
              </svg>
              <p>
                The line is the distance from the center. The circle marks
                radius 1.
              </p>
            </FunctionCard>
          )}
          {part !== 'length' && (
            <FunctionCard
              name="smoothstep(a, b, distance)"
              input={`a = −.025, b = .025; d = ${f(distance)}`}
              output={`s = ${f(smooth(distance))}`}
            >
              <div className="normalization">
                <code>t = clamp((d − a) / (b − a), 0, 1)</code>
                <p>
                  The edge transition runs from −.025 to .025. Subtract its
                  start, then divide by its width: t is the fraction of the way
                  across.
                </p>
                <output>t = {f(clamp((distance + 0.025) / 0.05))}</output>
              </div>
              <Curve
                fn={(t) => t * t * (3 - 2 * t)}
                value={clamp((distance + 0.025) / 0.05)}
                inputLabel="fraction across transition · t"
                outputLabel="eased fraction · s"
                linear
                onChange={(t) =>
                  onPick(...radialPoint(x, y, 1 - 0.025 + t * 0.05))
                }
              />
              <p>
                <code>s = 3t² − 2t³</code> bends the dashed straight ramp into
                an S. Its flat ends join smoothly to the constant values outside
                the transition.
              </p>
              <RangeControl
                label="Probe the edge"
                value={Math.max(-0.05, Math.min(0.05, distance))}
                min={-0.05}
                max={0.05}
                step={0.001}
                format={f}
                onChange={(d) => {
                  const scale = (1 + d) / (r || 1);
                  onPick(r ? x * scale : 1 + d, r ? y * scale : 0);
                }}
              />
              <p>
                The mask uses <code>1 − s = {f(1 - smooth(distance))}</code>.
                Inputs outside the graph stay at 0 or 1.
              </p>
            </FunctionCard>
          )}
        </>
      )}
      {step === 1 && (
        <>
          <FunctionCard
            name="dot(v, v)"
            input={`v = (${f(x)}, ${f(y)})`}
            output={`r² = ${f(squared)}`}
          >
            <svg
              className="function-plot"
              viewBox="0 0 280 170"
              role="img"
              aria-label="Squared horizontal and vertical distances add to the squared radius"
              {...dragDiagram((px, py) => {
                const n = clamp((py - 24) / 100);
                onPick(
                  px < 140 ? (Math.sign(x) || 1) * n : x,
                  px >= 140 ? (Math.sign(y) || 1) * n : y,
                );
              })}
            >
              <rect
                x="30"
                y="24"
                width={100 * Math.abs(x)}
                height={100 * Math.abs(x)}
                fill="var(--lab-accent)"
                opacity=".5"
              />
              <rect
                x="165"
                y="24"
                width={100 * Math.abs(y)}
                height={100 * Math.abs(y)}
                fill="var(--lab-accent)"
                opacity=".3"
              />
              <text x="30" y="144">
                x² = {f(x * x)}
              </text>
              <text x="165" y="144">
                y² = {f(y * y)}
              </text>
              <text x="140" y="75" textAnchor="middle">
                +
              </text>
            </svg>
            <p>A vector dotted with itself adds its squared components.</p>
          </FunctionCard>
          <FunctionCard
            name="sqrt(max(0, 1 − r²))"
            input={`1 − r² = ${f(1 - squared)}`}
            output={`height = ${f(Math.sqrt(Math.max(0, 1 - squared)))}`}
          >
            <Curve
              fn={(v) => Math.sqrt(Math.max(0, v))}
              value={Math.max(0, 1 - squared)}
              inputLabel="1 − r²"
              outputLabel="height"
              onChange={(n) => onPick(...radialPoint(x, y, Math.sqrt(1 - n)))}
            />
            <p>
              Taking the square root recovers height from height squared. At the
              center it is 1; at the edge it is 0.
            </p>
          </FunctionCard>
        </>
      )}
      {step === 2 && (
        <FunctionCard
          name="dot(v, direction)"
          input={`v = (${f(x)}, ${f(y)}); direction = (${f(dx)}, ${f(dy)})`}
          output={`projection = ${f(x * dx + y * dy)}`}
        >
          <div
            className="dot-products"
            role="img"
            aria-label="The dot product adds x times direction x and y times direction y"
          >
            {[x * dx, y * dy, x * dx + y * dy].map((n, i) => (
              <div key={i}>
                <code>{['x × direction.x', 'y × direction.y', 'sum'][i]}</code>
                <div className="dot-product-track">
                  <span
                    style={{
                      left: `${50 + Math.min(0, n) * 30}%`,
                      width: `${Math.abs(n) * 30}%`,
                      opacity: i === 2 ? 1 : 0.45,
                    }}
                  />
                </div>
                <output>{f(n)}</output>
              </div>
            ))}
            <p>Left of center is negative; right is positive.</p>
          </div>
          <p>
            The dot product is the signed length projected onto the direction.
          </p>
        </FunctionCard>
      )}
    </div>
  );
}
