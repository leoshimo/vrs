'use client';
/* oxlint-disable jsx-a11y/no-noninteractive-element-interactions -- Native drag target; move buttons provide the keyboard alternative. */
import type { RefObject } from 'react';
import type { ExpressionPlayer } from '../../../vrsjmp/src/avatar/expression-player';
import { Pin, X, ChevronLeft, ChevronRight } from 'lucide-react';
import {
  assignmentNames,
  assignmentLabels,
  type Assignment,
  type AvatarSetup,
  type AvatarExpression,
} from '@/lib/avatar/workspace';
import { ExpressionPreview, type PreviewCommand } from './ExpressionPreview';
export function ExpressionComparison({
  setup,
  expression: e,
  onDragStart,
  onDrop,
  onMove,
  canMoveLeft,
  canMoveRight,
  reference,
  input,
  heading,
  matches,
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
  expression?: AvatarExpression;
  onDragStart?: () => void;
  onDrop?: () => void;
  onMove?: (direction: number) => void;
  canMoveLeft?: boolean;
  canMoveRight?: boolean;
  reference: RefObject<ExpressionPlayer | null>;
  input: Assignment;
  heading?: 'Candidate' | 'Assigned';
  matches?: boolean;
  pinned?: boolean;
  selected?: boolean;
  dark: boolean;
  resolution: number;
  curves: boolean;
  command: PreviewCommand;
  working: boolean;
  shown: boolean;
  paused: boolean;
  onInspect: () => void;
  onPin: () => void;
  onClose?: () => void;
  onInput?: (input: Assignment) => void;
}) {
  const name = e?.name ?? 'No effect';
  const previewLabel = heading ? `${heading}: ${name}` : name;
  return (
    <figure
      className="pinned-preview"
      data-appearance={dark ? 'dark' : 'light'}
      data-selected={selected}
      data-matches={matches}
      aria-label={heading ? `${heading} preview` : `Pinned ${name}`}
      onDragOver={onDrop ? (event) => event.preventDefault() : undefined}
      onDrop={onDrop}
    >
      {heading && <span className="comparison-role">{heading}</span>}
      <div className="comparison-tools">
        {pinned && (
          <>
            <button
              aria-label={`Move ${name} left`}
              disabled={!canMoveLeft}
              onClick={() => onMove?.(-1)}
            >
              <ChevronLeft size={13} />
            </button>
            <button
              aria-label={`Move ${name} right`}
              disabled={!canMoveRight}
              onClick={() => onMove?.(1)}
            >
              <ChevronRight size={13} />
            </button>
          </>
        )}
        <button
          aria-label={`${pinned ? 'Unpin' : 'Pin'} preview ${previewLabel}`}
          aria-pressed={pinned}
          onClick={onPin}
        >
          <Pin size={13} />
        </button>
        {onClose && (
          <button aria-label={`Close preview ${name}`} onClick={onClose}>
            <X size={14} />
          </button>
        )}
      </div>
      <ExpressionPreview
        setup={setup}
        candidate={e}
        reference={reference}
        assignment={input}
        command={command}
        dark={dark}
        size={resolution}
        zoom={2.3}
        live
        paused={paused}
        curves={curves}
        working={working}
        shown={shown}
        label={`${previewLabel} preview`}
        onInspect={onInspect}
        overlay={matches && <span className="comparison-match">Assigned</span>}
        caption={
          <>
            <button
              className="pinned-preview-name"
              draggable={pinned}
              onDragStart={(event) => {
                event.dataTransfer.setData('text/plain', name);
                onDragStart?.();
              }}
              onClick={onInspect}
            >
              {name}
            </button>
            {onInput ? (
              <label className="comparison-input">
                <span>Assignment</span>
                <select
                  aria-label={`${name} assignment`}
                  value={input}
                  onChange={(event) =>
                    onInput(event.target.value as Assignment)
                  }
                >
                  {assignmentNames.map((a) => (
                    <option key={a} value={a}>
                      {assignmentLabels[a]}
                    </option>
                  ))}
                </select>
              </label>
            ) : (
              <div className="comparison-input">
                <span>{assignmentLabels[input]}</span>
              </div>
            )}
          </>
        }
      />
    </figure>
  );
}
