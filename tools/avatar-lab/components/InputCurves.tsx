'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Inline SVG signal graphs. */
import { useEffect, useRef } from 'react';
import {
  assignmentNames,
  assignmentLabels,
  type Assignment,
} from '@/lib/avatar/workspace';
import type { SignalTimeline } from '@/lib/avatar/signal-timeline';

export function InputCurves({
  source,
  assignment,
  status,
}: {
  source: SignalTimeline;
  assignment?: Assignment;
  status?: string;
}) {
  const plots = useRef<HTMLDivElement>(null);
  useEffect(
    () =>
      source.subscribe(() => {
        const right = source.samples.at(-1)?.time ?? 0;
        for (const path of plots.current?.querySelectorAll<SVGPathElement>(
          'path[data-channel]',
        ) ?? []) {
          const a = path.dataset.assignment as Assignment;
          const channel = path.dataset.channel as 'motion' | 'color';
          const d = source.samples
            .map(
              (sample, i) =>
                `${i ? 'L' : 'M'}${((sample.time - right + 4) * 40).toFixed(1)},${(25 - Math.min(2, sample.signals[a][channel]) * 10).toFixed(1)}`,
            )
            .join(' ');
          if (path.getAttribute('d') !== d) path.setAttribute('d', d);
        }
      }),
    [source, assignment],
  );
  const lanes = assignment ? [assignment] : assignmentNames;
  return (
    <div
      className="preview-chin"
      data-combined={!assignment}
      data-status={status !== undefined}
    >
      <div className="curve-legend" aria-hidden={Boolean(assignment)}>
        {!assignment && (
          <>
            <span>Motion</span>
            <span>Color</span>
          </>
        )}
      </div>
      <div className="curve-lanes" ref={plots}>
        {lanes.map((a) => (
          <div className="curve-lane" key={a}>
            <svg
              className="input-curves"
              viewBox="0 0 160 28"
              preserveAspectRatio="none"
              role="img"
              aria-label={`${assignmentLabels[a]} motion and color, last four seconds`}
            >
              <path d="M0 25H160" className="curve-baseline" />
              <path
                className="curve-motion"
                data-assignment={a}
                data-channel="motion"
              />
              <path
                className="curve-color"
                data-assignment={a}
                data-channel="color"
              />
            </svg>
            <span title={assignmentLabels[a]}>{assignmentLabels[a]}</span>
          </div>
        ))}
      </div>
      {status !== undefined && (
        <output className="preview-status" title={status}>
          {status || '\u00a0'}
        </output>
      )}
    </div>
  );
}
