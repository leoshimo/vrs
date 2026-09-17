'use client';
import { useEffect, useRef, useState } from 'react';
import { animate, useReducedMotion } from 'motion/react';
import { Plus, Play, Pause, Copy, X, SlidersHorizontal } from 'lucide-react';
import { useSetup } from './SetupProvider';
import { AvatarViewer } from './AvatarViewer';
import {
  AvatarControls,
  Choices,
  ExpressionControls,
  RangeControl,
  SelectControl,
} from './AvatarControls';
import { PreviewAppearance, usePreviewAppearance } from './PreviewAppearance';
import { Button } from './ui/button';
import { effects, expressionEffect } from '@/lib/avatar/avatar-effects';
import {
  groupedLibrary,
  variantName,
  workingEntry,
  motionAvailability,
} from '@/lib/avatar/expression-library';
import {
  compatible,
  compose,
  eventConfig,
  eventLabels,
  expression,
  type EventName,
  type Motion,
  type SavedExpression,
} from '@/lib/avatar/setup';
import type { MotionMode } from '@/lib/avatar/signal-motion';
const events: EventName[] = ['idle', 'typing', 'working', 'open', 'submit'];
type Selection =
  | { event: EventName }
  | { library: string }
  | { appearance: true };
export function Expressions() {
  const { setup, update, save, reload, dirty, status, ready } = useSetup();
  const [theme, setTheme] = usePreviewAppearance(),
    [selection, setSelection] = useState<Selection>({ event: 'idle' });
  const [flourish, setFlourish] = useState(false),
    [working, setWorking] = useState(false),
    [shown, setShown] = useState(true),
    [paused, setPaused] = useState(false),
    [query, setQuery] = useState('');
  const [view, setView] = useState('bar'),
    [zoom, setZoom] = useState('3');
  const [resolution, setResolution] = useState('96');
  const [state, setState] = useState<MotionMode>('idle'),
    [pulse, setPulse] = useState(0),
    [transient, setTransient] = useState<EventName | null>(null),
    [progress, setProgress] = useState(1),
    [dramatic, setDramatic] = useState(false),
    [libraryPlaying, setLibraryPlaying] = useState(false);
  const [entrance, setEntrance] = useState({
    appearanceMode: 'print',
    appearanceTarget: 'bar',
  });
  const [libraryOpen, setLibraryOpen] = useState(false),
    [adding, setAdding] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null),
    animation = useRef<ReturnType<typeof animate> | null>(null);
  const reduce = useReducedMotion();
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
      animation.current?.stop();
    },
    [],
  );
  const event =
    'event' in selection
      ? selection.event === 'open' && flourish
        ? 'flourish'
        : selection.event
      : null;
  const selected = event
    ? expression(setup, event)
    : 'library' in selection
      ? setup.expressions.find((e) => e.id === selection.library)
      : null;
  const library = groupedLibrary(setup);
  const selectedGroup =
    selected?.motions.length === 1
      ? library.find((g) => g.id === expressionEffect(selected.motions[0]))
      : undefined;
  const combinations = setup.expressions.filter((e) => e.motions.length > 1);
  const config =
    libraryPlaying && selected
      ? compose(setup.appearance, selected.motions)
      : eventConfig(setup, transient);
  if (dramatic && !libraryPlaying) {
    Object.assign(config, entrance);
  }
  function edit(fn: (e: SavedExpression) => SavedExpression) {
    if (selected)
      update((s) => ({
        ...s,
        expressions: s.expressions.map((e) =>
          e.id === selected.id ? fn(e) : e,
        ),
      }));
  }
  function patchMotion(index: number, patch: Partial<Motion>) {
    edit((e) => ({
      ...e,
      motions: e.motions.map((m, i) => (i === index ? { ...m, ...patch } : m)),
    }));
  }
  function clearTimer() {
    if (timer.current) clearTimeout(timer.current);
  }
  function hide() {
    clearTimer();
    animation.current?.stop();
    setShown(false);
    setTransient(null);
    setState('idle');
  }
  function appearance() {
    clearTimer();
    setSelection({ appearance: true });
    setLibraryPlaying(false);
    setState('idle');
    setTransient(null);
    setAdding(false);
  }
  function chooseLibrary(e: SavedExpression) {
    setSelection({ library: e.id });
    setAdding(false);
    runLibrary(e);
  }
  function trigger(which: EventName) {
    clearTimer();
    setPaused(false);
    setLibraryPlaying(false);
    setShown(true);
    if (which === 'working' || which === 'idle') {
      setWorking(which === 'working');
      setState('idle');
      setTransient(null);
      return;
    }
    setTransient(which);
    const e = expression(setup, which),
      mode = e.motions[0]?.action ?? 'idle';
    setState(mode === 'wake' ? 'idle' : mode);
    setPulse((p) => p + 1);
    if (which === 'open' || which === 'flourish') {
      const flourishNow = which === 'flourish';
      const reveal = flourishNow || mode === 'wake';
      setDramatic(reveal);
      setEntrance(
        flourishNow
          ? { appearanceMode: 'print', appearanceTarget: 'bar' }
          : {
              appearanceMode: e.motions[0]?.settings.appearanceMode ?? 'fill',
              appearanceTarget:
                e.motions[0]?.settings.appearanceTarget ?? 'avatar',
            },
      );
      animation.current?.stop();
      if (!reduce) {
        setProgress(0);
        animation.current = animate(0, 1, {
          duration:
            mode === 'wake' ? e.motions[0].duration : flourishNow ? 0.68 : 0.12,
          onUpdate: setProgress,
        });
      } else setProgress(1);
    }
    timer.current = setTimeout(
      () => {
        setTransient(null);
        setState('idle');
      },
      which === 'typing'
        ? 160
        : Math.max(
            1200,
            ...e.motions.map(
              (m) => (m.settings.surfaceDuration ?? m.duration) * 1000 + 800,
            ),
          ),
    );
  }
  function runLibrary(e: SavedExpression) {
    clearTimer();
    setTransient(null);
    setLibraryPlaying(true);
    setShown(true);
    setPaused(false);
    setDramatic(false);
    setProgress(1);
    const mode = e.motions[0]?.action ?? 'idle';
    setState(mode === 'wake' ? 'idle' : mode);
    setPulse((p) => p + 1);
    if (mode === 'wake' && !reduce) {
      setDramatic(true);
      setProgress(0);
      animation.current?.stop();
      animation.current = animate(0, 1, {
        duration: e.motions[0].duration,
        onUpdate: setProgress,
      });
    }
  }
  function chooseEvent(e: EventName) {
    clearTimer();
    setAdding(false);
    setSelection({ event: e });
    setLibraryPlaying(false);
    if (e === 'working') {
      setWorking(true);
      setState('idle');
      setTransient(null);
    } else {
      setWorking(false);
      if (e === 'typing' || e === 'submit' || e === 'open')
        trigger(e === 'open' && flourish ? 'flourish' : e);
      else {
        clearTimer();
        setTransient(null);
        setState('idle');
      }
    }
  }
  function duplicate() {
    if (!selected) return;
    const id = `custom-${crypto.randomUUID()}`;
    update((s) => ({
      ...s,
      expressions: [
        ...s.expressions,
        { ...structuredClone(selected), id, name: `${selected.name} copy` },
      ],
      events: event ? { ...s.events, [event]: id } : s.events,
    }));
    if (!event) setSelection({ library: id });
  }
  function addMotion(entry: SavedExpression) {
    if (!selected) return;
    const motion = structuredClone(entry.motions[0]);
    if (selected.motions.length === 1) {
      const id = `custom-${crypto.randomUUID()}`;
      update((s) => ({
        ...s,
        expressions: [
          ...s.expressions,
          {
            id,
            name: `${selected.name} + ${effects[expressionEffect(motion)].name}`,
            motions: [...structuredClone(selected.motions), motion],
          },
        ],
        events: event ? { ...s.events, [event]: id } : s.events,
      }));
      if (!event) setSelection({ library: id });
    } else edit((e) => ({ ...e, motions: [...e.motions, motion] }));
    setAdding(false);
  }
  const canAdd =
    selected &&
    selected.motions.length > 0 &&
    selected.motions.every((m) => m.action === 'loading');
  return (
    <section className="expression-editor">
      <header className="options-heading">
        <h1>Expressions</h1>
        <div className="file-actions">
          <output>{status || (dirty ? 'Unsaved' : 'Saved')}</output>
          <Button disabled={!ready} onClick={() => void reload()}>
            Reload file
          </Button>
          <Button
            disabled={!ready || !dirty || status === 'Saving…'}
            onClick={() => void save()}
            title="Save presets/vrs.json"
          >
            Save
          </Button>
        </div>
      </header>
      <section className="expression-preview" aria-label="Interactive preview">
        <div className="preview-toolbar">
          <Choices
            label="View"
            value={view}
            options={[
              ['avatar', 'Avatar'],
              ['bar', 'Bar'],
            ]}
            onChange={setView}
          />
          <PreviewAppearance theme={theme} onChange={setTheme} />
          <Button aria-pressed={'appearance' in selection} onClick={appearance}>
            <SlidersHorizontal size={14} /> Appearance
          </Button>
          {view === 'avatar' && (
            <Choices
              label="Zoom"
              value={zoom}
              options={[
                ['1', '1×'],
                ['2', '2×'],
                ['3', '3×'],
              ]}
              onChange={setZoom}
            />
          )}
          <Button
            className="preview-pause"
            aria-label={paused ? 'Play' : 'Pause'}
            onClick={() => {
              setPaused(!paused);
              if (paused) animation.current?.play();
              else animation.current?.pause();
            }}
          >
            {paused ? <Play size={14} /> : <Pause size={14} />}
          </Button>
        </div>
        <div className="avatar-canvas" data-view={view} data-appearance={theme}>
          <div
            className="interactive-avatar"
            inert={!shown}
            style={{
              opacity: shown ? 1 : 0,
              pointerEvents: shown ? 'auto' : 'none',
            }}
          >
            <AvatarViewer
              config={config}
              theme={theme}
              size={Number(resolution)}
              displaySize={42}
              zoom={Number(zoom)}
              bar={view === 'bar'}
              state={state}
              pulse={pulse}
              working={
                libraryPlaying
                  ? selected?.motions.some((m) => m.action === 'loading')
                  : working
              }
              appearance={dramatic ? progress : 1}
              windowOpacity={dramatic ? 1 : progress}
              animate={!reduce}
              active={shown && !paused}
              query={query}
              onQueryChange={setQuery}
              onType={() => trigger('typing')}
              onSubmit={() => trigger('submit')}
            />
          </div>
          {shown && (
            <span className="viewer-caption">
              {resolution} px
              {view === 'avatar' ? ` · ${zoom}×` : ' · 560 px bar'}
            </span>
          )}
        </div>
        <div className="event-transport">
          <label className="preview-visibility">
            <input
              type="checkbox"
              role="switch"
              aria-label="Visible"
              aria-checked={shown}
              checked={shown}
              onChange={(e) =>
                e.target.checked
                  ? trigger(flourish ? 'flourish' : 'open')
                  : hide()
              }
            />
            <span>Visible</span>
          </label>
          <label>
            <input
              type="checkbox"
              checked={flourish}
              onChange={(e) => setFlourish(e.target.checked)}
            />
            Flourish
          </label>
          <Button
            disabled={!shown}
            aria-pressed={working && !libraryPlaying}
            onClick={() => {
              setLibraryPlaying(false);
              setWorking(!working);
              setState('idle');
              setTransient(null);
            }}
          >
            Working
          </Button>
          <Button disabled={!shown} onClick={() => trigger('typing')}>
            Type
          </Button>
          <Button disabled={!shown} onClick={() => trigger('submit')}>
            Submit
          </Button>
          <output>
            {!shown
              ? ''
              : libraryPlaying
                ? selected?.name
                : working
                  ? `Working${transient ? ` · ${eventLabels[transient]}` : ''}`
                  : transient
                    ? eventLabels[transient]
                    : 'Idle'}
          </output>
        </div>
      </section>
      <div className="expression-body">
        <nav className="expression-nav" aria-label="Expression editor">
          <h2>Events</h2>
          {events.map((e) => (
            <button
              key={e}
              aria-current={
                'event' in selection && selection.event === e
                  ? 'page'
                  : undefined
              }
              onClick={() => chooseEvent(e)}
            >
              {eventLabels[e]}
              <small>
                {
                  expression(setup, e === 'open' && flourish ? 'flourish' : e)
                    .name
                }
              </small>
            </button>
          ))}
          <button
            className="library-toggle"
            aria-expanded={libraryOpen}
            onClick={() => setLibraryOpen(!libraryOpen)}
          >
            Library <span>{library.length}</span>
          </button>
          {libraryOpen && (
            <>
              {library.map((group) => (
                <button
                  key={group.id}
                  aria-current={
                    'library' in selection && selectedGroup?.id === group.id
                      ? 'page'
                      : undefined
                  }
                  onClick={() => chooseLibrary(group.entries[0])}
                >
                  {group.name}
                </button>
              ))}
              {combinations.length > 0 && (
                <>
                  <h2 className="combinations-heading">Combinations</h2>
                  {combinations.map((e) => (
                    <button
                      key={e.id}
                      aria-current={
                        'library' in selection && selection.library === e.id
                          ? 'page'
                          : undefined
                      }
                      onClick={() => chooseLibrary(e)}
                    >
                      {e.name}
                    </button>
                  ))}
                </>
              )}
            </>
          )}
        </nav>
        <section className="expression-parameters" aria-label="Parameters">
          {'appearance' in selection ? (
            <>
              <h2>Avatar appearance</h2>
              <div className="appearance-preview-settings">
                <Choices
                  label="Rendering size"
                  value={resolution}
                  options={[
                    ['42', '42 px'],
                    ['64', '64 px'],
                    ['96', '96 px'],
                  ]}
                  onChange={setResolution}
                />
                <Choices
                  label="Avatar zoom"
                  value={zoom}
                  options={[
                    ['1', '1×'],
                    ['2', '2×'],
                    ['3', '3×'],
                  ]}
                  onChange={(value) => {
                    setZoom(value);
                    setView('avatar');
                  }}
                />
              </div>
              <AvatarControls
                config={setup.appearance}
                onChange={(patch) =>
                  update((s) => ({
                    ...s,
                    appearance: { ...s.appearance, ...patch, shape: 'pearl' },
                  }))
                }
              />
              <Choices
                label="Bar theme"
                value={setup.appearance.uiSurface ?? 'porcelain'}
                options={[
                  ['porcelain', 'Neutral'],
                  ['solar', 'Warm'],
                  ['mineral', 'Cool'],
                ]}
                onChange={(uiSurface) =>
                  update((s) => ({
                    ...s,
                    appearance: { ...s.appearance, uiSurface },
                  }))
                }
              />
            </>
          ) : (
            selected && (
              <>
                <div className="parameter-heading">
                  <h2>
                    {event
                      ? eventLabels[event]
                      : (selectedGroup?.name ?? selected.name)}
                  </h2>
                  <Button
                    onClick={() =>
                      event ? trigger(event) : runLibrary(selected)
                    }
                  >
                    <Play size={13} /> Try
                  </Button>
                  <Button
                    onClick={duplicate}
                    title="Duplicate expression"
                    aria-label="Duplicate expression"
                  >
                    <Copy size={13} />
                  </Button>
                </div>
                {event && (
                  <SelectControl
                    label="Expression"
                    value={selected.id}
                    options={setup.expressions
                      .filter((e) => compatible(event, e))
                      .map((e) => [e.id, e.name])}
                    onChange={(id) => {
                      update((s) => ({
                        ...s,
                        events: { ...s.events, [event]: id },
                      }));
                    }}
                  />
                )}
                {!event &&
                  selectedGroup &&
                  selectedGroup.entries.length > 1 && (
                    <SelectControl
                      label="Variant"
                      value={selected.id}
                      options={selectedGroup.entries.map((e) => [
                        e.id,
                        variantName(e),
                      ])}
                      onChange={(id) =>
                        chooseLibrary(
                          setup.expressions.find((e) => e.id === id)!,
                        )
                      }
                    />
                  )}
                <label className="expression-name">
                  Name{' '}
                  <input
                    aria-label="Expression name"
                    value={selected.name}
                    onChange={(e) =>
                      edit((p) => ({ ...p, name: e.target.value }))
                    }
                  />
                </label>
                {selected.motions.length === 0 && (
                  <p className="quiet-note">Quick window entrance.</p>
                )}
                {selected.motions.map((m, i) => {
                  const f = expressionEffect(m);
                  const variants =
                    library
                      .find((g) => g.id === f)
                      ?.entries.filter(
                        (e) =>
                          e.motions[0].action === m.action &&
                          e.id === e.motions[0].preset,
                      ) ?? [];
                  return (
                    <section
                      className="motion-parameters"
                      key={`${m.preset}-${i}`}
                    >
                      <header>
                        <h3>{effects[f].name}</h3>
                        {selected.motions.length > 1 && (
                          <Button
                            aria-label={`Remove ${effects[f].name}`}
                            onClick={() =>
                              edit((e) => ({
                                ...e,
                                motions: e.motions.filter((_, n) => n !== i),
                              }))
                            }
                          >
                            <X size={13} />
                          </Button>
                        )}
                      </header>
                      {selected.motions.length > 1 && variants.length > 1 && (
                        <SelectControl
                          label="Preset"
                          value={m.preset}
                          options={variants.map((p) => [p.id, variantName(p)])}
                          onChange={(id) =>
                            patchMotion(
                              i,
                              structuredClone(
                                variants.find((e) => e.id === id)!.motions[0],
                              ),
                            )
                          }
                        />
                      )}
                      <ExpressionControls
                        action={m.action}
                        config={{ ...setup.appearance, ...m.settings }}
                        onChange={(patch) =>
                          patchMotion(i, {
                            settings: { ...m.settings, ...patch },
                          })
                        }
                        hideVariant
                      />
                      {m.action === 'wake' && (
                        <RangeControl
                          label="Duration"
                          value={m.duration}
                          min={0.1}
                          max={3}
                          step={0.1}
                          onChange={(duration) => patchMotion(i, { duration })}
                        />
                      )}
                    </section>
                  );
                })}
                {canAdd && (
                  <div className="add-motion">
                    <Button
                      aria-expanded={adding}
                      aria-controls="motion-picker"
                      onClick={() => setAdding(!adding)}
                    >
                      <Plus size={14} /> Add motion
                    </Button>
                    {adding && (
                      <section
                        id="motion-picker"
                        className="motion-picker"
                        aria-label="Add motion from library"
                      >
                        <p>Working combines continuous motions.</p>
                        <div>
                          {library.map((group) => {
                            const entry = workingEntry(group),
                              availability = motionAvailability(
                                group,
                                selected,
                              );
                            return (
                              <button
                                key={group.id}
                                disabled={!entry || availability === 'Added'}
                                title={
                                  availability === 'Working'
                                    ? `Add ${group.name}`
                                    : availability === 'Added'
                                      ? 'Already in this combination'
                                      : `${availability} motion`
                                }
                                onClick={() => {
                                  if (entry) addMotion(entry);
                                }}
                              >
                                <span>{group.name}</span>
                                <small>{availability}</small>
                              </button>
                            );
                          })}
                        </div>
                      </section>
                    )}
                  </div>
                )}
              </>
            )
          )}
        </section>
      </div>
    </section>
  );
}
