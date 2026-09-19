'use client';
import { useEffect, useSyncExternalStore } from 'react';
import {
  workspaceKey,
  makeWorkspace,
  readWorkspace,
  configuration,
  configurationPart,
  withConfigurationScope,
  type ConfigurationScope,
  type AvatarSetup,
} from '@/lib/avatar/workspace';
import type { AvatarSetup as Configuration } from '../../../vrsjmp/src/avatar/expression-model';

const defaults = makeWorkspace();
let state = {
  setup: defaults,
  saved: configuration(defaults),
  revision: '',
  ready: false,
  saving: null as ConfigurationScope | null,
  error: '',
};
let initialized = false;
let loading: Promise<void> | undefined;
const listeners = new Set<() => void>();
function publish(patch: Partial<typeof state>) {
  state = { ...state, ...patch };
  for (const notify of listeners) notify();
}
function snapshot() {
  if (!initialized) {
    initialized = true;
    try {
      state = {
        ...state,
        setup: readWorkspace(
          localStorage.getItem(workspaceKey) ??
            localStorage.getItem('avatar-lab-workspace-v4'),
        ),
      };
    } catch {
      /* Browser storage is optional. */
    }
  }
  return state;
}
function subscribe(notify: () => void) {
  listeners.add(notify);
  return () => {
    listeners.delete(notify);
  };
}
function update(fn: (setup: AvatarSetup) => AvatarSetup) {
  const setup = fn(snapshot().setup);
  try {
    localStorage.setItem(workspaceKey, JSON.stringify(setup));
  } catch {
    /* Keep the session's edits. */
  }
  publish({ setup, error: '' });
}
async function request(setup?: Configuration) {
  const response = await fetch('/__avatar/configuration', {
    method: setup ? 'PUT' : 'GET',
    ...(setup
      ? {
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            setup,
            revision: state.revision,
          }),
        }
      : {}),
  });
  if (!response.ok) throw new Error(await response.text());
  return (await response.json()) as { setup: Configuration; revision: string };
}
function load() {
  loading ??= request()
    .then(({ setup: saved, revision }) => {
      const unchanged =
        JSON.stringify(configuration(state.setup)) ===
        JSON.stringify(state.saved);
      if (unchanged) update((s) => ({ ...s, ...saved }));
      publish({ saved, revision, ready: true });
    })
    .catch(() =>
      publish({ error: 'Start the local lab server to save changes.' }),
    );
  return loading;
}
async function save(scope: 'assignments' | 'appearance') {
  if (state.saving || !state.ready) return false;
  const next = withConfigurationScope(state.saved, state.setup, scope);
  publish({ saving: scope, error: '' });
  try {
    const { setup: saved, revision } = await request(next);
    publish({ saved, revision });
    return true;
  } catch (error) {
    publish({
      error: error instanceof Error ? error.message : 'Could not save.',
    });
    return false;
  } finally {
    publish({ saving: null });
  }
}
const serverState = state;
export function useSetup() {
  const current = useSyncExternalStore(subscribe, snapshot, () => serverState);
  useEffect(() => {
    void load();
  }, []);
  return {
    ...current,
    update,
    save,
    modified: (scope: ConfigurationScope) =>
      JSON.stringify(configurationPart(current.setup, scope)) !==
      JSON.stringify(configurationPart(current.saved, scope)),
    revert: (scope: ConfigurationScope) =>
      update((s) => withConfigurationScope(s, state.saved, scope)),
  };
}
