'use client';
import { Pin, X } from 'lucide-react';
import {
  assignmentNames,
  assignmentLabels,
  type Assignment,
  type AvatarSetup,
} from '@/lib/avatar/workspace';
import type { Comparison } from '@/lib/avatar/comparison';
import { ExpressionPreview, type PreviewCommand } from './ExpressionPreview';
export function ExpressionComparison({
  setup,
  comparison,
  pinned,
  selected,
  dark,
  resolution,
  curves,
  command,
  working,
  shown,
  paused,
  onInspect,
  onPin,
  onClose,
  onInput,
}: {
  setup: AvatarSetup;
  comparison: Comparison;
  pinned: boolean;
  selected: boolean;
  dark: boolean;
  resolution: number;
  curves: boolean;
  command: PreviewCommand;
  working: boolean;
  shown: boolean;
  paused: boolean;
  onInspect: () => void;
  onPin: () => void;
  onClose: () => void;
  onInput: (input: Assignment) => void;
}) {
  const e = setup.expressions.find((e) => e.id === comparison.id);
  const name = e?.name ?? 'No effect';
  return (
    <figure
      className="pinned-preview"
      data-appearance={dark ? 'dark' : 'light'}
      data-selected={selected}
    >
      <div className="comparison-tools">
        <button
          aria-label={`${pinned ? 'Unpin' : 'Pin'} preview ${name}`}
          aria-pressed={pinned}
          onClick={onPin}
        >
          <Pin size={13} />
        </button>
        <button aria-label={`Close preview ${name}`} onClick={onClose}>
          <X size={14} />
        </button>
      </div>
      <ExpressionPreview
        setup={setup}
        candidate={e}
        assignment={comparison.input}
        command={command}
        dark={dark}
        size={resolution}
        zoom={2.3}
        live
        paused={paused}
        curves={curves}
        working={working}
        shown={shown}
        label={`${name} preview`}
        onInspect={onInspect}
        caption={
          <>
            <button className="pinned-preview-name" onClick={onInspect}>
              {name}
            </button>
            <label className="comparison-input">
              <span>Input</span>
              <select
                aria-label={`${name} input`}
                value={comparison.input}
                onChange={(event) => onInput(event.target.value as Assignment)}
              >
                {assignmentNames.map((a) => (
                  <option key={a} value={a}>
                    {assignmentLabels[a]}
                  </option>
                ))}
              </select>
            </label>
          </>
        }
      />
    </figure>
  );
}
