'use client';
import { useSyncExternalStore } from 'react';
import { Expressions } from './Expressions';
import { Recipe } from './Recipe';
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
  return (
    <div className="lab" data-page={page}>
      <header className="lab-header">
        <span className="lab-title">Avatar Lab</span>
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
