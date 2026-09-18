'use client';
import { useSyncExternalStore } from 'react';
import {
  workspaceKey as draftKey,
  makeWorkspace,
  readWorkspace,
  type AvatarSetup as Setup,
} from '@/lib/avatar/workspace';

const defaults = makeWorkspace();
let draft: Setup | undefined;
const listeners = new Set<() => void>();
function snapshot() {
  if (!draft) {
    try {
      draft = readWorkspace(
        localStorage.getItem(draftKey),
        localStorage.getItem('avatar-lab-draft-v1'),
        localStorage.getItem('avatar-lab-workspace-v2'),
        localStorage.getItem('avatar-lab-workspace-v3'),
      );
    } catch {
      draft = makeWorkspace();
    }
  }
  return draft;
}
function onStorage(event: StorageEvent) {
  if (event.key !== draftKey && event.key !== null) return;
  draft = readWorkspace(event.key === null ? null : event.newValue);
  for (const notify of listeners) notify();
}
function subscribe(notify: () => void) {
  if (!listeners.size) window.addEventListener('storage', onStorage);
  listeners.add(notify);
  return () => {
    listeners.delete(notify);
    if (!listeners.size) window.removeEventListener('storage', onStorage);
  };
}
function update(fn: (setup: Setup) => Setup) {
  draft = fn(snapshot());
  try {
    localStorage.setItem(draftKey, JSON.stringify(draft));
  } catch {
    // Keep edits in this session if browser storage is unavailable.
  }
  for (const notify of listeners) notify();
}

export function useSetup() {
  const setup = useSyncExternalStore(subscribe, snapshot, () => defaults);
  return { setup, update };
}
