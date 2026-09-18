'use client';
import { useMemo, useState } from 'react';
import { Play, Pause, Search } from 'lucide-react';
import { useSetup } from './useSetup';
import { usePlayback, type PlaybackMode } from './usePlayback';
import { AvatarControls, Choices, SelectControl } from './AvatarControls';
import { PreviewAppearance, usePreviewAppearance } from './PreviewAppearance';
import { ExpressionPreview } from './ExpressionPreview';
import { ExpressionComparison } from './ExpressionComparison';
import { ExpressionRow } from './ExpressionRow';
import { ExpressionInspector } from './ExpressionInspector';
import { Button } from './ui/button';
import { expressionGroups } from '@/lib/avatar/expression-groups';
import { routineNames } from '@/lib/avatar/playback';
import {
  comparisonKey,
  defaultInput,
  type Comparison,
} from '@/lib/avatar/comparison';
import {
  assigned,
  assignmentNames,
  assignmentLabels,
  motionNames,
  makeWorkspace,
  type Assignment,
  type AvatarExpression,
} from '@/lib/avatar/workspace';

export function Expressions() {
  const { setup, update } = useSetup();
  const [theme, setTheme] = usePreviewAppearance();
  const playback = usePlayback(setup);
  const [tab, setTab] = useState<'appearance' | 'assignments'>('assignments');
  const [assignment, setAssignment] = useState<Assignment>('typing');
  const [selection, setSelection] = useState<Comparison>();
  const [temporary, setTemporary] = useState<Comparison | null>(null);
  const [pins, setPins] = useState<Comparison[]>([]);
  const [section, setSection] = useState('Color'),
    [search, setSearch] = useState('');
  const [view, setView] = useState('bar'),
    [resolution, setResolution] = useState('96'),
    [zoom, setZoom] = useState('3');
  const defaults = useMemo(() => makeWorkspace(), []);
  const current = selection ?? {
    id: setup.assignments[assignment],
    input: assignment,
  };
  const selected = setup.expressions.find((e) => e.id === current.id);
  const isAssigned = current.id === setup.assignments[assignment];
  const isPinned = (p: Comparison) =>
    pins.some((pin) => comparisonKey(pin) === comparisonKey(p));
  const previews = [
    ...pins,
    ...(temporary && !isPinned(temporary) ? [temporary] : []),
  ];
  function inspect(p: Comparison) {
    setTab('assignments');
    setSelection(p);
    setTemporary(p);
  }
  function chooseAssignment(a: Assignment) {
    setAssignment(a);
    inspect({ id: setup.assignments[a], input: a });
    setSearch('');
  }
  function select(e?: AvatarExpression) {
    const existing = pins.find((p) => p.id === (e?.id ?? null));
    inspect(
      existing ?? {
        id: e?.id ?? null,
        input: e ? defaultInput(e) : assignment,
      },
    );
  }
  function togglePin(p: Comparison) {
    if (isPinned(p))
      setPins((list) =>
        list.filter((pin) => comparisonKey(pin) !== comparisonKey(p)),
      );
    else setPins((list) => [...list, p]);
  }
  function close(p: Comparison) {
    setPins((list) =>
      list.filter((pin) => comparisonKey(pin) !== comparisonKey(p)),
    );
    if (temporary && comparisonKey(temporary) === comparisonKey(p))
      setTemporary(null);
  }
  function changeInput(p: Comparison, input: Assignment) {
    const next = { ...p, input };
    setPins((list) => {
      const changed = list.map((pin) =>
        comparisonKey(pin) === comparisonKey(p) ? next : pin,
      );
      return changed.filter(
        (pin, i) =>
          changed.findIndex(
            (other) => comparisonKey(other) === comparisonKey(pin),
          ) === i,
      );
    });
    inspect(next);
  }
  function assign() {
    update((s) => ({
      ...s,
      assignments: { ...s.assignments, [assignment]: current.id },
    }));
    inspect({ ...current, input: assignment });
  }
  function edit(e: AvatarExpression) {
    update((s) => ({
      ...s,
      expressions: s.expressions.map((old) => (old.id === e.id ? e : old)),
    }));
  }
  const original = defaults.expressions.find((e) => e.id === selected?.id);
  const modified =
    selected &&
    original &&
    JSON.stringify(selected) !== JSON.stringify(original);
  const groups = expressionGroups(
    setup.expressions.filter((e) =>
      `${e.name} ${e.description} ${e.patterns.map((p) => motionNames[p.kind]).join(' ')}`
        .toLowerCase()
        .includes(search.toLowerCase()),
    ),
    assignment,
  );

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
      <div className="workspace-previews">
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
            />
          </div>
        </div>
        {previews.map((p) => (
          <ExpressionComparison
            key={comparisonKey(p)}
            setup={setup}
            comparison={p}
            pinned={isPinned(p)}
            selected={comparisonKey(p) === comparisonKey(current)}
            dark={theme === 'dark'}
            resolution={Number(resolution)}
            curves
            command={playback.command}
            working={playback.working}
            shown={playback.shown}
            paused={playback.paused}
            onInspect={() => inspect(p)}
            onPin={() => togglePin(p)}
            onClose={() => close(p)}
            onInput={(input) => changeInput(p, input)}
          />
        ))}
      </div>
      <div className="workspace-transport">
        <fieldset className="manual-inputs" aria-label="Manual input">
          <legend>Input</legend>
          <Button
            onClick={() => playback.act(playback.shown ? 'hide' : 'open')}
          >
            {playback.shown ? 'Hide' : 'Open'}
          </Button>
          <Button onClick={() => playback.act('openAfterIdle')}>
            Open after idle
          </Button>
          <Button
            disabled={!playback.shown}
            onClick={() => playback.act('typing')}
          >
            Type
          </Button>
          <Button
            disabled={!playback.shown}
            onClick={() => playback.act('submit')}
          >
            Submit
          </Button>
          <Button
            disabled={!playback.shown}
            aria-pressed={playback.working}
            onClick={() => playback.act(playback.working ? 'idle' : 'working')}
          >
            {playback.working ? 'Idle' : 'Work'}
          </Button>
        </fieldset>
        <fieldset className="routine-controls" aria-label="Playback">
          <legend>Playback</legend>
          <select
            aria-label="Playback"
            value={playback.mode}
            onChange={(event) =>
              playback.chooseMode(event.target.value as PlaybackMode)
            }
          >
            <option value="manual">Manual</option>
            {routineNames.map(([id, label]) => (
              <option key={id} value={id}>
                {label}
              </option>
            ))}
          </select>
          <Button
            aria-label={playback.paused ? 'Play playback' : 'Pause playback'}
            title={playback.paused ? 'Play' : 'Pause'}
            onClick={playback.togglePlay}
          >
            {playback.paused ? <Play size={14} /> : <Pause size={14} />}
          </Button>
        </fieldset>
      </div>
      <nav className="workspace-tabs" aria-label="Editor sections">
        {(['assignments', 'appearance'] as const).map((t) => (
          <Button key={t} aria-pressed={tab === t} onClick={() => setTab(t)}>
            {t === 'assignments' ? 'Assignments' : 'Appearance'}
          </Button>
        ))}
      </nav>
      <div className="workspace-body" data-tab={tab}>
        <nav
          className="workspace-navigation"
          aria-label={tab === 'assignments' ? 'Assignments' : 'Appearance'}
        >
          {tab === 'assignments'
            ? assignmentNames.map((a) => (
                <button
                  key={a}
                  aria-current={assignment === a ? 'page' : undefined}
                  data-running={playback.active.includes(a)}
                  onClick={() => chooseAssignment(a)}
                >
                  <span className="assignment-name">{assignmentLabels[a]}</span>
                  <small>{assigned(setup, a)?.name ?? 'No effect'}</small>
                </button>
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
        </nav>
        {tab === 'appearance' ? (
          <section className="workspace-appearance">
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
            <section className="workspace-gallery" key={assignment}>
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
                    selected={current.id === null}
                    assigned={setup.assignments[assignment] === null}
                    onSelect={() => select()}
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
                      const p =
                        current.id === e.id
                          ? current
                          : (pins.find((p) => p.id === e.id) ?? {
                              id: e.id,
                              input: defaultInput(e),
                            });
                      return (
                        <ExpressionRow
                          key={e.id}
                          name={e.name}
                          description={e.description}
                          selected={current.id === e.id}
                          assigned={setup.assignments[assignment] === e.id}
                          pinned={isPinned(p)}
                          onSelect={() => select(e)}
                          onPin={() => {
                            togglePin(p);
                            inspect(p);
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
            >
              <div className="inspector-heading">
                <h2>{selected?.name ?? 'No effect'}</h2>
              </div>
              <Button disabled={isAssigned} onClick={assign}>
                {isAssigned
                  ? `Assigned to ${assignmentLabels[assignment]}`
                  : `Assign to ${assignmentLabels[assignment]}`}
              </Button>
              {selected && (
                <>
                  <ExpressionInspector
                    key={selected.id}
                    expression={selected}
                    onChange={edit}
                    sustained={
                      current.input === 'idle' || current.input === 'working'
                    }
                  />
                  {modified && (
                    <button
                      className="reset-settings"
                      onClick={() => edit(structuredClone(original!))}
                    >
                      Reset settings
                    </button>
                  )}
                </>
              )}
            </aside>
          </>
        )}
      </div>
    </section>
  );
}
