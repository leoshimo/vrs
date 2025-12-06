'use client';
import { useSyncExternalStore } from 'react';
import { Button } from '@/components/ui/button';

export type StudyTheme = 'light' | 'dark';
const storageKey = 'vrsjmp-study-appearance';
const changeEvent = 'vrsjmp-study-appearance-change';
let current: StudyTheme = 'dark';

function subscribe(notify: () => void) {
  window.addEventListener(changeEvent, notify);
  window.addEventListener('storage', notify);
  return () => {
    window.removeEventListener(changeEvent, notify);
    window.removeEventListener('storage', notify);
  };
}
function snapshot(): StudyTheme {
  try {
    const saved = localStorage.getItem(storageKey);
    if (saved === 'light' || saved === 'dark') current = saved;
  } catch {
    // The toggle still works in memory when browser storage is unavailable.
  }
  return current;
}
function setTheme(theme: StudyTheme) {
  current = theme;
  try {
    localStorage.setItem(storageKey, theme);
  } catch {
    // Keep this session's choice even if it cannot be saved.
  }
  window.dispatchEvent(new Event(changeEvent));
}
export function usePreviewAppearance() {
  const theme = useSyncExternalStore(
    subscribe,
    snapshot,
    () => 'dark' as const,
  );
  return [theme, setTheme] as const;
}

export function PreviewAppearance({
  theme,
  onChange,
}: {
  theme: StudyTheme;
  onChange: (theme: StudyTheme) => void;
}) {
  return (
    <fieldset className="study-choices">
      <legend>Preview</legend>
      <div>
        {(['light', 'dark'] as const).map((value) => (
          <Button
            key={value}
            type="button"
            aria-pressed={theme === value}
            onClick={() => onChange(value)}
          >
            {value === 'light' ? 'Light' : 'Dark'}
          </Button>
        ))}
      </div>
    </fieldset>
  );
}
