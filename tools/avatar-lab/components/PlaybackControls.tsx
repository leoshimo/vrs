'use client';
import { useRef, useState, type CSSProperties } from 'react';
import { Play, Pause, ChevronDown, Sparkles } from 'lucide-react';
import { ResetTweaks } from './ResetTweaks';
import {
  rails,
  sequenceNames,
  paintCell,
  cellAt,
  cellSeconds,
  type Rail,
  type SequenceName,
} from '@/lib/avatar/sequence';
import type { usePlayback } from './usePlayback';
const labels = {
  window: 'Window',
  typing: 'Type',
  submit: 'Submit',
  working: 'Work',
  complete: 'Complete',
};
export function PlaybackControls({
  playback,
}: {
  playback: ReturnType<typeof usePlayback>;
}) {
  const { progressRef, ...p } = playback;
  const [expanded, setExpanded] = useState(false);
  const [afterIdle, setAfterIdle] = useState(false);
  const paint = useRef<{ rail: Rail; on: boolean } | null>(null);
  const count = cellAt(p.sequence.duration);
  const editing = expanded && !p.manual;
  const action = (rail: Rail) => {
    if (rail === 'window')
      p.act(p.shown ? 'hide' : afterIdle ? 'openAfterIdle' : 'open');
    else if (rail === 'working') p.act(p.working ? 'idle' : 'working');
    else p.act(rail);
  };
  return (
    <section
      className="sequence"
      data-expanded={editing}
      data-running={p.running}
      data-manual={p.manual}
      aria-label="Sequence"
    >
      <div className="sequence-toolbar">
        <span className="sequence-heading">Sequence</span>
        <label className="sequence-picker">
          <select
            aria-label="Sequence preset"
            value={p.manual ? 'manual' : p.name}
            onChange={(e) =>
              p.chooseSequence(e.target.value as SequenceName | 'manual')
            }
          >
            <option value="manual">Manual</option>
            {Object.entries(sequenceNames).map(([id, title]) => (
              <option key={id} value={id}>
                {title}
              </option>
            ))}
          </select>
        </label>
        <div className="sequence-controls">
          <button
            className="sequence-icon"
            aria-label={p.running ? 'Pause sequence' : 'Play sequence'}
            title={p.running ? 'Pause' : 'Play'}
            onClick={p.togglePlay}
            disabled={p.manual}
          >
            {p.running ? <Pause size={16} /> : <Play size={16} />}
          </button>
          <button
            className="sequence-edit"
            aria-expanded={editing}
            disabled={p.manual}
            onClick={() => setExpanded(!expanded)}
          >
            Edit <ChevronDown size={14} />
          </button>
          {p.edited && (
            <ResetTweaks
              onClick={p.resetSequence}
              title="Reset sequence"
              label="Edited"
            />
          )}
          <label className="sequence-duration">
            <input
              aria-label="Sequence duration in seconds"
              type="number"
              min="2"
              max="30"
              step={cellSeconds}
              disabled={p.manual}
              value={p.sequence.duration}
              onChange={(e) => {
                const n = Number(e.target.value);
                if (n >= 2 && n <= 30)
                  p.editSequence({
                    ...p.sequence,
                    duration: cellAt(n) * cellSeconds,
                  });
              }}
            />{' '}
            s
          </label>
        </div>
      </div>
      <div
        className="sequence-rails"
        ref={progressRef}
        style={{ '--cells': count } as CSSProperties}
        onPointerUp={() => {
          paint.current = null;
        }}
        onPointerCancel={() => {
          paint.current = null;
        }}
      >
        {rails.map((rail) => (
          <div className="sequence-rail" key={rail}>
            <div className="sequence-label">
              <button
                disabled={rail !== 'window' && !p.shown}
                onClick={() => action(rail)}
                aria-pressed={
                  rail === 'window'
                    ? p.shown
                    : rail === 'working'
                      ? p.working
                      : undefined
                }
                data-active={
                  rail === 'window' ? p.shown : p.active.includes(rail)
                }
                title={
                  rail === 'window'
                    ? p.shown
                      ? 'Hide'
                      : afterIdle
                        ? 'Open after idle'
                        : 'Open'
                    : undefined
                }
              >
                {labels[rail]}
              </button>
              {rail === 'window' && (
                <button
                  className="sequence-icon"
                  aria-label="Use Open after idle"
                  aria-pressed={afterIdle}
                  data-active={afterIdle}
                  title="Use Open after idle for the Window button"
                  onClick={() => setAfterIdle((value) => !value)}
                >
                  <Sparkles size={13} />
                </button>
              )}
            </div>
            <div className="sequence-track">
              {Array.from({ length: p.manual ? 0 : count }, (_, cell) => {
                const on = p.sequence.cells[rail].includes(cell);
                const entrance =
                  rail === 'window' &&
                  on &&
                  !p.sequence.cells.window.includes(cell - 1);
                return (
                  <div className="sequence-cell" data-on={on} key={cell}>
                    <button
                      tabIndex={editing ? 0 : -1}
                      disabled={!editing}
                      aria-pressed={on}
                      aria-label={`${labels[rail]} at ${cell * cellSeconds} seconds`}
                      title={`${cell * cellSeconds} s`}
                      onPointerDown={(e) => {
                        if (e.button !== 0) return;
                        paint.current = { rail, on: !on };
                        p.editSequence(paintCell(p.sequence, rail, cell, !on));
                      }}
                      onPointerEnter={(e) => {
                        if (e.buttons === 1 && paint.current?.rail === rail)
                          p.editSequence(
                            paintCell(p.sequence, rail, cell, paint.current.on),
                          );
                      }}
                      onClick={(e) => {
                        if (e.detail === 0)
                          p.editSequence(
                            paintCell(p.sequence, rail, cell, !on),
                          );
                      }}
                    />
                    {entrance && (
                      <button
                        className="sequence-entrance"
                        tabIndex={editing ? 0 : -1}
                        disabled={!editing}
                        title={
                          p.sequence.entrances[cell] === 'openAfterIdle'
                            ? 'Open after idle'
                            : 'Open'
                        }
                        aria-label={`Entrance at ${cell * cellSeconds} seconds: ${p.sequence.entrances[cell] === 'openAfterIdle' ? 'Open after idle' : 'Open'}`}
                        onClick={() =>
                          p.editSequence({
                            ...p.sequence,
                            entrances: {
                              ...p.sequence.entrances,
                              [cell]:
                                p.sequence.entrances[cell] === 'openAfterIdle'
                                  ? 'open'
                                  : 'openAfterIdle',
                            },
                          })
                        }
                      >
                        {p.sequence.entrances[cell] === 'openAfterIdle' ? (
                          <Sparkles size={10} />
                        ) : (
                          <span />
                        )}
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        ))}
        <div className="sequence-playhead-area" aria-hidden="true">
          <span className="sequence-playhead" />
        </div>
      </div>
      {editing && (
        <div className="sequence-ruler">
          <span />
          <div>
            <span>0 s</span>
            <span>{p.sequence.duration / 2} s</span>
            <span>{p.sequence.duration} s</span>
          </div>
        </div>
      )}
    </section>
  );
}
