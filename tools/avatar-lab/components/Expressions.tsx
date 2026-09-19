'use client';
import { PlaybackControls } from './PlaybackControls';
import { useEffect, useRef, useState } from 'react';
import { ArrowLeft, Search, X } from 'lucide-react';
import { ResetTweaks } from './ResetTweaks';
import { SaveAssignments } from './SaveAssignments';
import { useSetup } from './useSetup';
import { usePlayback } from './usePlayback';
import { useMediaQuery } from './useMediaQuery';
import { AvatarControls, Choices, SelectControl } from './AvatarControls';
import { PreviewAppearance, usePreviewAppearance } from './PreviewAppearance';
import type { ExpressionPlayer } from '../../../vrsjmp/src/avatar/expression-player';
import { ExpressionPreview } from './ExpressionPreview';
import { ExpressionComparison } from './ExpressionComparison';
import { ExpressionRow } from './ExpressionRow';
import { ExpressionActions } from './ExpressionActions';
import { ExpressionInspector } from './ExpressionInspector';
import { Button } from './ui/button';
import { expressionGroups } from '@/lib/avatar/expression-groups';
import type { Comparison } from '@/lib/avatar/comparison';
import {
  assigned,
  assignmentNames,
  assignmentLabels,
  motionNames,
  candidateKey,
  makeCandidate,
  sameExpression,
  applyCandidate,
  type Assignment,
  type AvatarExpression,
} from '@/lib/avatar/workspace';

export function Expressions() {
  const {
    setup,
    saved,
    update,
    save,
    revert,
    modified: unsaved,
    ready,
    saving,
    error,
  } = useSetup();
  const [theme, setTheme] = usePreviewAppearance();
  const playback = usePlayback(setup);
  const reference = useRef<ExpressionPlayer | null>(null);
  const previews = useRef<HTMLDivElement>(null);
  const editor = useRef<HTMLDivElement>(null);
  const inspectorOpener = useRef<HTMLElement | null>(null);
  const inspectorClose = useRef<HTMLButtonElement>(null);
  const compact = useMediaQuery('(max-width: 1050px)');
  const singleColumn = useMediaQuery('(max-width: 760px)');
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [tab, setTab] = useState<'appearance' | 'assignments'>('assignments');
  const [assignment, setAssignment] = useState<Assignment>('typing');
  const [selectedId, setSelectedId] = useState<string | null>();
  const [pins, setPins] = useState<Comparison[]>([]);
  const [section, setSection] = useState('Color'),
    [search, setSearch] = useState('');
  const [view, setView] = useState('bar'),
    [resolution, setResolution] = useState('96'),
    [zoom, setZoom] = useState('3');
  const applied = setup.assignments[assignment];
  const id = selectedId === undefined ? (applied?.id ?? null) : selectedId;
  const seed =
    applied?.id === id
      ? applied
      : (setup.expressions.find((e) => e.id === id) ?? null);
  const key = candidateKey(assignment, id);
  const candidate = setup.candidates[key] ?? makeCandidate(seed);
  const selected = candidate.expression;
  const modified = !sameExpression(selected, candidate.original);
  const matches = sameExpression(selected, applied);

  function closeInspector() {
    setInspectorOpen(false);
    requestAnimationFrame(() =>
      inspectorOpener.current?.focus({ preventScroll: true }),
    );
  }
  function revealInspector() {
    inspectorOpener.current = document.activeElement as HTMLElement;
    setInspectorOpen(true);
    if (compact && editor.current && previews.current) {
      const offset =
        editor.current.getBoundingClientRect().top -
        previews.current.getBoundingClientRect().height -
        12;
      if (offset > 0) window.scrollBy({ top: offset });
    }
    previews.current
      ?.querySelector('.pinned-preview[data-selected="true"]')
      ?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }
  useEffect(() => {
    if (!compact || !inspectorOpen) return;
    inspectorClose.current?.focus({ preventScroll: true });
    function escape(event: KeyboardEvent) {
      if (
        event.key !== 'Escape' ||
        event.defaultPrevented ||
        document.querySelector('dialog[open]')
      )
        return;
      event.preventDefault();
      setInspectorOpen(false);
      requestAnimationFrame(() =>
        inspectorOpener.current?.focus({ preventScroll: true }),
      );
    }
    document.addEventListener('keydown', escape);
    return () => document.removeEventListener('keydown', escape);
  }, [compact, inspectorOpen]);

  function inspect(
    expression: AvatarExpression | null,
    input = assignment,
    fresh = false,
  ) {
    setTab('assignments');
    setAssignment(input);
    setSelectedId(expression?.id ?? null);
    const key = candidateKey(input, expression?.id ?? null);
    if (fresh || !setup.candidates[key])
      update((s) => ({
        ...s,
        candidates: { ...s.candidates, [key]: makeCandidate(expression) },
      }));
  }
  function chooseAssignment(input: Assignment) {
    inspect(setup.assignments[input], input);
    setSearch('');
    setInspectorOpen(false);
  }
  function editEffect(
    expression: AvatarExpression | null,
    input = assignment,
    fresh = false,
  ) {
    inspect(expression, input, fresh);
    revealInspector();
  }
  function edit(expression: AvatarExpression) {
    update((s) => ({
      ...s,
      candidates: { ...s.candidates, [key]: { ...candidate, expression } },
    }));
  }
  const isPinned = (expression: AvatarExpression | null, input = assignment) =>
    pins.some(
      (p) => p.input === input && sameExpression(p.expression, expression),
    );
  function togglePin(expression: AvatarExpression | null, input = assignment) {
    setPins((list) => {
      const found = list.find(
        (p) => p.input === input && sameExpression(p.expression, expression),
      );
      return found
        ? list.filter((p) => p.key !== found.key)
        : [
            ...list,
            {
              key: crypto.randomUUID(),
              input,
              expression: structuredClone(expression),
            },
          ];
    });
  }
  const dragPin = useRef<string | null>(null);
  function movePin(key: string, target: string) {
    setPins((list) => {
      const from = list.findIndex((p) => p.key === key),
        to = list.findIndex((p) => p.key === target);
      if (from < 0 || to < 0 || from === to) return list;
      const next = [...list];
      next.splice(to, 0, next.splice(from, 1)[0]);
      return next;
    });
  }
  const groups = expressionGroups(
    setup.expressions.filter((e) =>
      `${e.name} ${e.description} ${e.patterns.map((p) => motionNames[p.kind]).join(' ')}`
        .toLowerCase()
        .includes(search.toLowerCase()),
    ),
    assignment,
  );
  const previewProps = {
    setup,
    reference,
    dark: theme === 'dark',
    resolution: Number(resolution),
    curves: true,
    command: playback.command,
    working: playback.working,
    shown: playback.shown,
    paused: playback.paused,
  };
  return (
    <section className="expression-workspace">
      <div className="workspace-toolbar">
        <Choices
          label="View"
          value={view}
          options={[
            ['bar', 'Bar'],
            ['avatar', 'Avatar'],
          ]}
          onChange={setView}
        />
        <PreviewAppearance
          label="Appearance"
          theme={theme}
          onChange={setTheme}
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
      </div>
      <div className="workspace-previews" ref={previews}>
        <div
          className="workspace-main-preview"
          data-view={view}
          data-appearance={theme}
        >
          <div>
            <ExpressionPreview
              setup={setup}
              command={playback.command}
              dark={theme === 'dark'}
              size={Number(resolution)}
              zoom={Number(zoom)}
              bar={view === 'bar'}
              live
              paused={playback.paused}
              curves
              status={playback.status}
              working={playback.working}
              shown={playback.shown}
              query={playback.query}
              onQueryChange={playback.setQuery}
              onAction={playback.act}
              onSample={(player) => {
                reference.current = player;
              }}
            />
          </div>
        </div>
        <ExpressionComparison
          {...previewProps}
          heading="Assigned"
          input={assignment}
          expression={applied ?? undefined}
          onInspect={() => editEffect(applied, assignment, true)}
          onPin={() => togglePin(applied)}
        />
        <ExpressionComparison
          {...previewProps}
          heading="Candidate"
          matches={matches}
          input={assignment}
          expression={selected ?? undefined}
          selected
          onInspect={() => {
            setTab('assignments');
            revealInspector();
          }}
          onPin={() => togglePin(selected)}
        />
        {pins.map((p, index) => (
          <ExpressionComparison
            {...previewProps}
            key={p.key}
            input={p.input}
            expression={p.expression ?? undefined}
            pinned
            onDragStart={() => {
              dragPin.current = p.key;
            }}
            onDrop={() => {
              if (dragPin.current) movePin(dragPin.current, p.key);
              dragPin.current = null;
            }}
            onMove={(direction) => {
              const target = pins[index + direction];
              if (target) movePin(p.key, target.key);
            }}
            canMoveLeft={index > 0}
            canMoveRight={index < pins.length - 1}
            onInspect={() => editEffect(p.expression, p.input, true)}
            onPin={() =>
              setPins((list) => list.filter((pin) => pin.key !== p.key))
            }
            onClose={() =>
              setPins((list) => list.filter((pin) => pin.key !== p.key))
            }
            onInput={(input) =>
              setPins((list) =>
                list.map((pin) =>
                  pin.key === p.key ? { ...pin, input } : pin,
                ),
              )
            }
          />
        ))}
      </div>
      <PlaybackControls playback={playback} />
      <div className="workspace-editor-heading">
        <nav className="workspace-tabs" aria-label="Editor sections">
          {(['assignments', 'appearance'] as const).map((t) => (
            <Button key={t} aria-pressed={tab === t} onClick={() => setTab(t)}>
              {t === 'assignments' ? 'Assignments' : 'Appearance'}
            </Button>
          ))}
        </nav>
      </div>
      {error && (
        <p className="configuration-error" role="alert">
          {error}
        </p>
      )}
      <div className="workspace-body" data-tab={tab} ref={editor}>
        <nav
          className="workspace-navigation"
          aria-label={tab === 'assignments' ? 'Assignments' : 'Appearance'}
        >
          <div className="assignment-list">
            {tab === 'assignments'
              ? assignmentNames.map((a) => (
                  <div
                    className="assignment-row"
                    key={a}
                    data-running={playback.active.includes(a)}
                  >
                    <button
                      aria-current={assignment === a ? 'page' : undefined}
                      onClick={() => chooseAssignment(a)}
                    >
                      <span className="assignment-name">
                        {assignmentLabels[a]}
                      </span>
                      <small>{assigned(setup, a)?.name ?? 'No effect'}</small>
                    </button>
                    {unsaved(a) && (
                      <ResetTweaks
                        action="Revert"
                        title={`Revert ${assignmentLabels[a]}`}
                        onClick={() => revert(a)}
                      />
                    )}
                  </div>
                ))
              : ['Color', 'Field', 'Print'].map((name) => (
                  <button
                    key={name}
                    aria-current={section === name ? 'page' : undefined}
                    onClick={() => setSection(name)}
                  >
                    {name}
                  </button>
                ))}
          </div>
          {tab === 'assignments' && (
            <SaveAssignments
              modified={unsaved('assignments')}
              ready={ready}
              saving={Boolean(saving)}
              error={error}
              onSave={() => save('assignments')}
            />
          )}
        </nav>
        {tab === 'appearance' ? (
          <section className="workspace-appearance">
            <div className="configuration-heading">
              <span>Appearance</span>
              <div className="configuration-actions">
                <button
                  disabled={!unsaved('appearance') || Boolean(saving)}
                  onClick={() => revert('appearance')}
                >
                  Revert appearance
                </button>
                <button
                  disabled={!unsaved('appearance') || !ready || Boolean(saving)}
                  onClick={() => void save('appearance')}
                >
                  {saving === 'appearance' ? 'Saving…' : 'Save appearance'}
                </button>
              </div>
            </div>
            <div className="appearance-rendering">
              <SelectControl
                label="Resolution"
                value={resolution}
                options={[
                  ['42', '42 px'],
                  ['64', '64 px'],
                  ['96', '96 px'],
                ]}
                onChange={setResolution}
              />
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
            </div>
            <AvatarControls
              section={section}
              config={setup.appearance}
              defaultConfig={saved.appearance}
              onChange={(patch) =>
                update((s) => ({
                  ...s,
                  appearance: { ...s.appearance, ...patch, shape: 'pearl' },
                }))
              }
            />
          </section>
        ) : (
          <>
            <section
              className="workspace-gallery"
              key={assignment}
              inert={singleColumn && inspectorOpen}
            >
              <label className="expression-search">
                <Search size={15} />
                <input
                  type="search"
                  aria-label="Search expressions"
                  placeholder="Search expressions"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
              </label>
              <div className="expression-list">
                {!search && (
                  <ExpressionRow
                    name="No effect"
                    description="Adds no motion or color."
                    assignmentLabel={assignmentLabels[assignment]}
                    selected={selected === null}
                    matches={applied === null}
                    pinned={isPinned(null)}
                    onPin={() => togglePin(null)}
                    onApply={() => {
                      update((s) =>
                        applyCandidate(s, assignment, makeCandidate(null)),
                      );
                      inspect(null);
                    }}
                    onSelect={() => editEffect(null)}
                  />
                )}
                {groups.map((group) => (
                  <section
                    className="expression-group"
                    key={group.name}
                    aria-label={group.name}
                  >
                    <h3>{group.name}</h3>
                    {group.expressions.map((e) => {
                      const starting = applied?.id === e.id ? applied : e;
                      const rowCandidate =
                        setup.candidates[
                          candidateKey(assignment, starting?.id ?? null)
                        ] ?? makeCandidate(starting);
                      const expression = rowCandidate.expression;
                      return (
                        <ExpressionRow
                          key={e.id}
                          name={e.name}
                          description={e.description}
                          selected={selected?.id === e.id}
                          matches={sameExpression(applied, expression)}
                          assignmentLabel={assignmentLabels[assignment]}
                          pinned={isPinned(expression)}
                          onSelect={() => editEffect(starting)}
                          onPin={() => togglePin(expression)}
                          onApply={() => {
                            update((s) =>
                              applyCandidate(s, assignment, rowCandidate),
                            );
                            inspect(expression);
                          }}
                        />
                      );
                    })}
                  </section>
                ))}
              </div>
              {!groups.length && (
                <p className="empty-gallery">No expressions found.</p>
              )}
            </section>
            <aside
              className="workspace-inspector"
              aria-label="Expression settings"
              data-open={inspectorOpen}
              inert={compact && !inspectorOpen}
              aria-hidden={compact && !inspectorOpen}
            >
              <div className="inspector-header">
                <div className="inspector-context">
                  <span>{assignmentLabels[assignment]}</span>
                  <button
                    ref={inspectorClose}
                    className="inspector-close"
                    aria-label="Back to effects"
                    title="Close settings"
                    onClick={closeInspector}
                  >
                    <ArrowLeft size={14} className="inspector-back-icon" />
                    <span>Effects</span>
                    <X size={15} className="inspector-close-icon" />
                  </button>
                </div>
                <div className="inspector-heading">
                  <h2 title={selected?.name ?? 'No effect'}>
                    {selected?.name ?? 'No effect'}
                  </h2>
                  {modified && (
                    <ResetTweaks
                      title="Reset candidate tweaks"
                      onClick={() =>
                        update((s) => ({
                          ...s,
                          candidates: {
                            ...s.candidates,
                            [key]: makeCandidate(candidate.original),
                          },
                        }))
                      }
                    />
                  )}
                  <ExpressionActions
                    name={selected?.name ?? 'No effect'}
                    assignmentLabel={assignmentLabels[assignment]}
                    matches={matches}
                    pinned={isPinned(selected)}
                    onApply={() =>
                      update((s) => applyCandidate(s, assignment, candidate))
                    }
                    onPin={() => togglePin(selected)}
                  />
                </div>
              </div>
              <div className="inspector-content">
                {selected && (
                  <ExpressionInspector
                    key={key}
                    expression={selected}
                    original={candidate.original ?? undefined}
                    onChange={edit}
                    sustained={
                      assignment === 'idle' || assignment === 'working'
                    }
                  />
                )}
              </div>
            </aside>
          </>
        )}
      </div>
    </section>
  );
}
