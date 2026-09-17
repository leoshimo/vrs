'use client';
import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import defaults from '../presets/vrs.json';
import {
  motionFromPreset,
  validateSetup,
  type Setup,
} from '@/lib/avatar/setup';
import { expressionSettings } from '@/lib/avatar/avatar-expressions';
import { defaultSignal } from '@/lib/avatar/signal-field';
type Context = {
  setup: Setup;
  update: (fn: (s: Setup) => Setup) => void;
  save: () => Promise<void>;
  reload: () => Promise<void>;
  dirty: boolean;
  status: string;
  ready: boolean;
};
const Context = createContext<Context | null>(null);
function migrate(setup: Setup): Setup {
  if (setup.initialized) return setup;
  const next = structuredClone(setup);
  try {
    const previous = JSON.parse(
      localStorage.getItem('vrsjmp-avatar-pass18') || 'null',
    );
    if (previous)
      next.appearance = {
        ...next.appearance,
        ...previous,
        shape: 'pearl',
        effects: { ...next.appearance.effects, placeholder: false },
      };
    delete next.appearance.effectMix;
    delete next.appearance.effectLayerSettings;
    delete next.appearance.loopRipple;
    const overrides = JSON.parse(
      localStorage.getItem('vrsjmp-expression-overrides-v1') || '{}',
    );
    for (const e of next.expressions)
      for (const m of e.motions)
        if (overrides[m.preset]) {
          m.settings = expressionSettings(m.action, {
            ...defaultSignal,
            ...m.settings,
            ...overrides[m.preset].settings,
          });
          m.duration = overrides[m.preset].duration ?? m.duration;
        }
    const saved = JSON.parse(
      localStorage.getItem('vrsjmp-expressions') || '[]',
    );
    for (const old of saved) {
      const template = next.expressions.find(
        (e) => e.motions[0]?.action === old.action,
      )?.motions[0];
      if (template && !next.expressions.some((e) => e.id === old.id))
        next.expressions.push({
          id: old.id,
          name: old.name,
          motions: [
            {
              ...motionFromPreset(template.preset),
              settings: expressionSettings(old.action, {
                ...defaultSignal,
                ...old.settings,
              }),
            },
          ],
        });
    }
    const workingRipple = next.expressions
      .find((e) => e.id === 'working')
      ?.motions.find((m) => m.preset === 'surface-working');
    if (workingRipple)
      Object.assign(workingRipple.settings, {
        surfaceAmplitude: 0.28,
        surfaceDuration: 5,
      });
    validateSetup(next);
    return next;
  } catch {
    return setup;
  }
}
export function SetupProvider({ children }: { children: ReactNode }) {
  const [setup, setSetup] = useState(defaults as unknown as Setup),
    [baseline, setBaseline] = useState(JSON.stringify(defaults)),
    [revision, setRevision] = useState(''),
    [status, setStatus] = useState('Loading…'),
    [ready, setReady] = useState(false);
  const current = useRef(setup);
  useEffect(() => {
    current.current = setup;
  }, [setup]);
  async function reload() {
    try {
      const r = await fetch('/__avatar/setup');
      if (!r.ok) throw new Error('Could not read preset file');
      const data = await r.json();
      validateSetup(data.setup);
      setSetup(migrate(data.setup));
      setBaseline(JSON.stringify(data.setup));
      setRevision(data.revision);
      setReady(true);
      setStatus('');
    } catch (e) {
      setStatus(e instanceof Error ? e.message : 'Could not load');
    }
  }
  useEffect(() => {
    let cancelled = false;
    fetch('/__avatar/setup')
      .then(async (r) => {
        if (!r.ok) throw new Error('Could not read preset file');
        return r.json();
      })
      .then((data) => {
        if (cancelled) return;
        validateSetup(data.setup);
        setSetup(migrate(data.setup));
        setBaseline(JSON.stringify(data.setup));
        setRevision(data.revision);
        setReady(true);
        setStatus('');
      })
      .catch((e) => {
        if (!cancelled)
          setStatus(e instanceof Error ? e.message : 'Could not load');
      });
    return () => {
      cancelled = true;
    };
  }, []);
  async function save() {
    const snapshot = { ...current.current, initialized: true };
    setStatus('Saving…');
    try {
      const r = await fetch('/__avatar/setup', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ setup: snapshot, revision }),
      });
      const data = await r.json();
      if (!r.ok) throw new Error(data.error);
      setRevision(data.revision);
      setBaseline(JSON.stringify(snapshot));
      setSetup((now) => ({ ...now, initialized: true }));
      setStatus('Saved');
    } catch (e) {
      setStatus(e instanceof Error ? e.message : 'Save failed');
    }
  }
  const dirty =
    JSON.stringify({ ...setup, initialized: true }) !==
      JSON.stringify({ ...JSON.parse(baseline), initialized: true }) ||
    !setup.initialized;
  useEffect(() => {
    if (!dirty) return;
    const warn = (e: BeforeUnloadEvent) => {
      e.preventDefault();
    };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [dirty]);
  return (
    <Context.Provider
      value={{
        setup,
        update: (fn) => {
          setSetup(fn);
          setStatus('');
        },
        save,
        reload,
        dirty,
        status,
        ready,
      }}
    >
      {children}
    </Context.Provider>
  );
}
export function useSetup() {
  const value = useContext(Context);
  if (!value) throw new Error('Missing setup');
  return value;
}
