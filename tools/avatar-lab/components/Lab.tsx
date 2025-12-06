'use client';
import { useSyncExternalStore } from 'react';
import { SetupProvider } from './SetupProvider';
import { Expressions } from './Expressions';
import { Recipe } from './Recipe';
import { Choices } from './AvatarControls';
import { useMediaQuery } from './useMediaQuery';
type Theme = 'system' | 'light' | 'dark';
let sessionTheme: Theme = 'system';
function subscribePage(notify: () => void) {
  window.addEventListener('hashchange', notify);
  return () => window.removeEventListener('hashchange', notify);
}
function pageSnapshot() {
  return ['#recipe', '#avatar-recipe', '#anatomy'].includes(location.hash)
    ? 'recipe'
    : 'expressions';
}
function subscribeTheme(notify: () => void) {
  window.addEventListener('avatar-lab-theme-change', notify);
  window.addEventListener('storage', notify);
  return () => {
    window.removeEventListener('avatar-lab-theme-change', notify);
    window.removeEventListener('storage', notify);
  };
}
function themeSnapshot(): Theme {
  try {
    const saved = localStorage.getItem('avatar-lab-theme');
    if (saved === 'light' || saved === 'dark' || saved === 'system')
      sessionTheme = saved;
  } catch {
    /* Use the session preference. */
  }
  return sessionTheme;
}
export function Lab() {
  const page = useSyncExternalStore(
    subscribePage,
    pageSnapshot,
    () => 'expressions',
  );
  const appearance = useSyncExternalStore(
    subscribeTheme,
    themeSnapshot,
    () => 'system' as const,
  );
  const systemDark = useMediaQuery('(prefers-color-scheme: dark)');
  const dark = appearance === 'dark' || (appearance === 'system' && systemDark);
  return (
    <div
      className={`lab${dark ? ' dark' : ''}`}
      data-theme={dark ? 'dark' : 'light'}
    >
      <header className="lab-header">
        <nav aria-label="Tools">
          <a
            href="#expressions"
            aria-current={page === 'expressions' ? 'page' : undefined}
          >
            Expressions
          </a>
          <a
            href="#recipe"
            aria-current={page === 'recipe' ? 'page' : undefined}
          >
            Recipe
          </a>
        </nav>
        <Choices
          label="Page"
          value={appearance}
          options={[
            ['system', 'System'],
            ['light', 'Light'],
            ['dark', 'Dark'],
          ]}
          onChange={(value) => {
            sessionTheme = value;
            try {
              localStorage.setItem('avatar-lab-theme', value);
            } catch {
              /* Optional persistence. */
            }
            window.dispatchEvent(new Event('avatar-lab-theme-change'));
          }}
        />
      </header>
      <SetupProvider>
        <main>{page === 'recipe' ? <Recipe /> : <Expressions />}</main>
      </SetupProvider>
    </div>
  );
}
