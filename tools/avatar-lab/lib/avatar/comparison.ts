import {
  assignmentNames,
  previewAssignment,
  type Assignment,
  type AvatarExpression,
  type AvatarSetup,
} from './workspace';
import type { PlaybackAction } from '../../../../vrsjmp/src/avatar/expression-player';

export type Comparison = { id: string | null; input: Assignment };
export const comparisonKey = (p: Comparison) => `${p.input}:${p.id ?? 'none'}`;
export function comparisonAction(
  input: Assignment,
  action: PlaybackAction | 'reset' | 'resetHidden',
): PlaybackAction | null {
  if (input === 'working' && ['idle', 'reset', 'resetHidden'].includes(action))
    return 'idle';
  return input === action ? input : null;
}
export function defaultInput(e: AvatarExpression): Assignment {
  return e.patterns.every((p) => p.kind === 'lava')
    ? 'idle'
    : previewAssignment(e);
}
export function comparisonSetup(
  setup: AvatarSetup,
  p: Comparison,
): AvatarSetup {
  return {
    ...setup,
    assignments: Object.fromEntries(
      assignmentNames.map((a) => [a, a === p.input ? p.id : null]),
    ) as AvatarSetup['assignments'],
  };
}
export function assignmentComparisons(setup: AvatarSetup): Comparison[] {
  return assignmentNames.flatMap((input) => {
    const id = setup.assignments[input];
    return id ? [{ id, input }] : [];
  });
}
