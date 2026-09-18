'use client';
import { useSyncExternalStore } from 'react';
import { Expressions } from './Expressions';
import { Recipe } from './Recipe';
import { useMediaQuery } from './useMediaQuery';
function subscribePage(notify: () => void) {
  window.addEventListener('hashchange', notify);
  return () => window.removeEventListener('hashchange', notify);
}
function pageSnapshot() {
  return ['#recipe', '#avatar-recipe', '#anatomy'].includes(location.hash)
    ? 'recipe'
    : 'expressions';
}
export function Lab() {
  const page = useSyncExternalStore(
    subscribePage,
    pageSnapshot,
    () => 'expressions',
  );
  const dark = useMediaQuery('(prefers-color-scheme: dark)');
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
      </header>
      <main>{page === 'recipe' ? <Recipe /> : <Expressions />}</main>
    </div>
  );
}
