'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Inspectable SVG diagrams. */
import { dragDiagram } from './diagramDrag';
import { useState } from 'react';
import {
  bayerThreshold,
  paletteCoordinate,
  palettePigment,
  rgbCSS,
  toneBand,
} from '@/lib/avatar/field-explainer';
export function GradientLesson({
  x,
  y,
  angle,
  strength,
  dark,
  onPick,
}: {
  x: number;
  y: number;
  angle: number;
  strength: number;
  dark: boolean;
  onPick?: (x: number, y: number) => void;
}) {
  const a = (angle * Math.PI) / 180,
    dx = Math.cos(a),
    dy = Math.sin(a),
    projection = x * dx + y * dy;
  const px = 120 + x * 80,
    py = 115 + y * 80;
  return (
    <div className="gradient-lesson">
      <svg
        viewBox="0 0 390 235"
        role="img"
        aria-label="Project a position onto the ink direction arrow"
        {...dragDiagram(
          onPick
            ? (px, py) => onPick((px - 120) / 80, (py - 115) / 80)
            : undefined,
        )}
      >
        <circle
          cx="120"
          cy="115"
          r="80"
          fill="none"
          stroke="currentColor"
          opacity=".25"
        />
        <line
          x1={120 - dy * 90}
          y1={115 + dx * 90}
          x2={120 + dy * 90}
          y2={115 - dx * 90}
          stroke="currentColor"
          strokeDasharray="3 5"
          opacity=".3"
        />
        <line
          x1={120 - dx * 95}
          y1={115 - dy * 95}
          x2={120 + dx * 100}
          y2={115 + dy * 100}
          stroke="var(--lab-accent)"
          strokeWidth="2"
        />
        <circle
          cx={120 + dx * 100}
          cy={115 + dy * 100}
          r="4"
          fill="var(--lab-accent)"
        />
        <line
          x1={px}
          y1={py}
          x2={120 + dx * projection * 80}
          y2={115 + dy * projection * 80}
          stroke="currentColor"
          strokeDasharray="3 3"
        />
        <line
          x1="120"
          y1="115"
          x2={120 + dx * projection * 80}
          y2={115 + dy * projection * 80}
          stroke="var(--lab-accent)"
          strokeWidth="5"
        />
        <circle cx={px} cy={py} r="5" fill="currentColor" />
        <text x="240" y="70">
          ● chosen position
        </text>
        <text x="240" y="100">
          Dashed line: projection
        </text>
        <text x="240" y="130">
          Thick line: dot product
        </text>
        <text x="240" y="160">
          Arrow tip: more ink
        </text>
        <text x="48" y="220">
          Perpendicular to the arrow → zero change
        </text>
      </svg>
      <div className="sample-equation">
        <span>
          Position
          <b>
            ({x.toFixed(2)}, {y.toFixed(2)})
          </b>
        </span>
        <span>·</span>
        <span>
          Direction
          <b>
            ({dx.toFixed(2)}, {dy.toFixed(2)})
          </b>
        </span>
        <span>=</span>
        <span>
          Projection<b>{projection.toFixed(2)}</b>
        </span>
        <span>→</span>
        <span>
          Added coverage
          <b>
            {projection >= 0 ? '+' : ''}
            {(projection * 0.34 * strength * 100).toFixed(1)}%
          </b>
        </span>
      </div>
      <p>
        Project the point perpendicularly onto the arrow’s line. The signed
        distance along that line is the dot product. Positive values add ink;
        negative values remove ink. More coverage appears{' '}
        {dark ? 'brighter' : 'darker'} in this preview.
      </p>
    </div>
  );
}
export function PrintingLesson({
  density,
  pitch,
  matrix,
  dark,
}: {
  density: number;
  pitch: number;
  matrix: number;
  dark: boolean;
}) {
  const [cell, setCell] = useState(0);
  const index = cell % (matrix * matrix),
    threshold = bayerThreshold(
      index % matrix,
      Math.floor(index / matrix),
      matrix,
    );
  const tile = matrix * pitch;
  return (
    <div className="printing-lesson">
      <div className="lesson-columns">
        <figure>
          <figcaption>1 · Ink coverage</figcaption>
          <div
            className="density-sample"
            data-appearance={dark ? 'dark' : 'light'}
          >
            <div
              className="density-swatch"
              style={{
                background: dark ? '#e4e7df' : '#344134',
                opacity: density,
              }}
            />
          </div>
          <b>{Math.round(density * 100)}%</b>
          <p>At 50%, half the cells print.</p>
        </figure>
        <figure>
          <figcaption>2 · Threshold tile · enlarged</figcaption>
          <div
            className="threshold-grid"
            style={{ gridTemplateColumns: `repeat(${matrix},1fr)` }}
          >
            {Array.from({ length: matrix * matrix }, (_, i) => {
              const t = bayerThreshold(
                i % matrix,
                Math.floor(i / matrix),
                matrix,
              );
              return (
                <button
                  key={i}
                  aria-pressed={i === index}
                  onClick={() => setCell(i)}
                  title={`${(t * 100).toFixed(2)}%`}
                  style={{
                    background: `color-mix(in srgb,currentColor ${t * 18}%,transparent)`,
                  }}
                >
                  {Math.round(t * 100)}
                </button>
              );
            })}
          </div>
          <p>
            Numbers are minimum coverage percentages. Each threshold is used
            once per tile.
          </p>
        </figure>
        <figure>
          <figcaption>3 · Repeated print</figcaption>
          <svg
            className="print-patch"
            viewBox="0 0 96 96"
            role="img"
            aria-label="Fixed-size patch showing actual cell size"
          >
            <defs>
              <pattern
                id={`print-tile-${matrix}`}
                width={tile}
                height={tile}
                patternUnits="userSpaceOnUse"
              >
                {Array.from({ length: matrix * matrix }, (_, i) =>
                  density >=
                  bayerThreshold(i % matrix, Math.floor(i / matrix), matrix) ? (
                    <rect
                      key={i}
                      x={(i % matrix) * pitch}
                      y={Math.floor(i / matrix) * pitch}
                      width={pitch}
                      height={pitch}
                      fill="currentColor"
                    />
                  ) : null,
                )}
              </pattern>
            </defs>
            <rect width="96" height="96" fill={`url(#print-tile-${matrix})`} />
          </svg>
          <p>Fixed 96 × 96 area. Larger cells enlarge the pattern.</p>
        </figure>
      </div>
      <div className="sample-equation">
        <span>
          Coverage<b>{(density * 100).toFixed(1)}%</b>
        </span>
        <span>{density >= threshold ? '≥' : '<'}</span>
        <span>
          Chosen threshold<b>{(threshold * 100).toFixed(1)}%</b>
        </span>
        <span>→</span>
        <strong>
          {density >= threshold ? 'Print this cell' : 'Leave this cell empty'}
        </strong>
      </div>
      <p>
        The tile repeats every {matrix} × {pitch.toFixed(1)} ={' '}
        <b>{tile.toFixed(1)} logical pixels</b>. Cell size changes texture
        scale; it does not change the proportion of cells that print. Matrix
        size changes the arrangement and number of available thresholds. Bayer
        is a standard ordered-dithering method; this shader computes its
        thresholds directly.
      </p>
    </div>
  );
}
export function ColorLesson({
  x,
  y,
  phase,
  density,
  bands,
  saturation,
  instrument,
  dark,
  showBands = true,
  onPick,
}: {
  showBands?: boolean;
  onPick?: (x: number, y: number) => void;
  x: number;
  y: number;
  phase: number;
  density: number;
  bands: number;
  saturation: number;
  instrument: boolean;
  dark: boolean;
}) {
  const t = paletteCoordinate(x, y, phase),
    rgb = palettePigment(t, instrument, saturation, dark),
    level = toneBand(density, bands);
  const colors = Array.from({ length: 33 }, (_, i) =>
    rgbCSS(palettePigment(i / 32, instrument, saturation, dark)),
  );
  return (
    <div className="color-lesson">
      <div className="lesson-columns">
        <figure>
          <figcaption>1 · Position + clock</figcaption>
          <div className="number-inputs">
            <span>
              x <b>{x.toFixed(2)}</b>
            </span>
            <span>
              y <b>{y.toFixed(2)}</b>
            </span>
            <span>
              phase <b>{phase.toFixed(2)}</b>
            </span>
          </div>
          <code>0.5 + 0.5 × sin(x × 1.8 + y × 0.9 + phase × 0.7)</code>
          <p>
            Sine turns an increasing number into a smooth back-and-forth value
            between 0 and 1.
          </p>
        </figure>
        <figure>
          <figcaption>2 · Palette lookup</figcaption>
          <div
            className="palette-lookup"
            style={{ background: `linear-gradient(90deg,${colors.join(',')})` }}
          >
            <i style={{ left: `${t * 100}%` }} />
          </div>
          <b>Lookup position: {t.toFixed(3)}</b>
          <p>
            0 is the first pigment, 0.5 the middle, 1 the last. In-between
            values blend neighboring pigments.
          </p>
        </figure>
        <figure>
          <figcaption>3 · Pigment at this point</figcaption>
          <div className="pigment-swatch" style={{ background: rgbCSS(rgb) }} />
          <p>
            {rgbCSS(rgb)}
            <br />
            This is the palette pigment. The avatar then blends it with its base
            ink using color strength and activity.
          </p>
        </figure>
      </div>
      <figure className="palette-map">
        <figcaption>
          Where the colors land · same clock, different positions
        </figcaption>
        <svg
          viewBox="0 0 200 100"
          role="img"
          aria-label="Palette color varies smoothly across coordinates"
          {...dragDiagram(
            onPick ? (px, py) => onPick(px / 100 - 1, py / 50 - 1) : undefined,
          )}
        >
          {Array.from({ length: 800 }, (_, i) => {
            const px = (i % 40) / 20 - 1,
              py = Math.floor(i / 40) / 10 - 1;
            return (
              <rect
                key={i}
                x={(i % 40) * 5}
                y={Math.floor(i / 40) * 5}
                width="5"
                height="5"
                fill={rgbCSS(
                  palettePigment(
                    paletteCoordinate(px, py, phase),
                    instrument,
                    saturation,
                    dark,
                  ),
                )}
              />
            );
          })}
          <circle
            cx={(x + 1) * 100}
            cy={(y + 1) * 50}
            r="3"
            fill="none"
            stroke="white"
            strokeWidth="1.5"
          />
        </svg>
      </figure>
      {showBands && (
        <>
          <h3>Tone bands are a separate operation</h3>
          <div className="tone-bins">
            {Array.from({ length: bands || 16 }, (_, i) => {
              const d = (i + 0.5) / (bands || 16),
                l = toneBand(d, bands);
              return (
                <div
                  key={i}
                  style={{
                    background: rgbCSS(rgb),
                    opacity: bands ? 0.3 + 0.7 * l : 1,
                  }}
                >
                  <span>
                    {bands
                      ? `${Math.round((i / bands) * 100)}–${Math.round(((i + 1) / bands) * 100)}%`
                      : ''}
                  </span>
                </div>
              );
            })}
          </div>
          <div className="sample-equation">
            <span>
              Coverage<b>{(density * 100).toFixed(1)}%</b>
            </span>
            <span>→</span>
            <span>
              {bands ? 'Assigned level' : 'No quantization'}
              <b>{level.toFixed(2)}</b>
            </span>
            <span>→</span>
            <span>
              Pigment / paper mix
              <b>
                {bands ? Math.round((0.3 + 0.7 * level) * 100) : 100}% pigment
              </b>
            </span>
          </div>
          <p>
            Bands classify <em>coverage</em>, not position. Points anywhere in
            the avatar with the same coverage fall into the same bin. The shader
            mixes that level into coverage at 50% strength, and uses it to vary
            pigment against the paper. Saturation separately blends each pigment
            toward a gray of similar brightness.
          </p>
        </>
      )}
    </div>
  );
}
