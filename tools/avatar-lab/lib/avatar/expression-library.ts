import {
  effects,
  expressionEffect,
  expressionUses,
  presetName,
  type EffectId,
} from './avatar-effects';
import { expressions } from './avatar-expressions';
import type { SavedExpression, Setup } from './setup';

export function groupedLibrary(setup: Setup) {
  return (Object.keys(effects) as EffectId[])
    .map((id) => ({
      id,
      name: effects[id].name,
      entries: setup.expressions.filter(
        (e) => e.motions.length === 1 && expressionEffect(e.motions[0]) === id,
      ),
    }))
    .filter((group) => group.entries.length);
}
export type LibraryGroup = ReturnType<typeof groupedLibrary>[number];
export function variantName(entry: SavedExpression) {
  const original = expressions.find((p) => p.id === entry.id);
  return original ? presetName(original) : entry.name;
}
export function workingEntry(group: LibraryGroup) {
  const entries = group.entries.filter(
    (e) => e.motions[0].action === 'loading',
  );
  return entries.find((e) => e.id === 'restless') ?? entries[0];
}
export function motionAvailability(
  group: LibraryGroup,
  selected: SavedExpression,
) {
  if (selected.motions.some((m) => expressionEffect(m) === group.id))
    return 'Added';
  if (workingEntry(group)) return 'Working';
  return [
    ...new Set(group.entries.map((e) => expressionUses[e.motions[0].action])),
  ].join(' / ');
}
