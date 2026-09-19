import catalog from './library.json' with { type: 'json' };
import { assignedSetup } from '../../../../vrsjmp/src/avatar/assigned-setup';
import {
  isAvatarSetup,
  isExpression,
} from '../../../../vrsjmp/src/avatar/setup-validation';
import {
  assignmentNames,
  type Assignment,
  type AvatarSetup as AssignedSetup,
  type AvatarExpression,
} from '../../../../vrsjmp/src/avatar/expression-model';
export * from '../../../../vrsjmp/src/avatar/expression-model';

// The catalog belongs to the lab. Applications need only the assigned setup.
export type Candidate = {
  original: AvatarExpression | null;
  expression: AvatarExpression | null;
};
export type AvatarSetup = AssignedSetup & {
  expressions: AvatarExpression[];
  candidates: Record<string, Candidate>;
};
export const candidateKey = (input: Assignment, id: string | null) =>
  `${input}:${id ?? 'none'}`;
export const sameExpression = (
  a: AvatarExpression | null,
  b: AvatarExpression | null,
) => JSON.stringify(a) === JSON.stringify(b);
export function makeCandidate(expression: AvatarExpression | null): Candidate {
  return {
    original: structuredClone(expression),
    expression: structuredClone(expression),
  };
}
export function applyCandidate(
  setup: AvatarSetup,
  input: Assignment,
  candidate: Candidate,
): AvatarSetup {
  return {
    ...setup,
    assignments: {
      ...setup.assignments,
      [input]: structuredClone(candidate.expression),
    },
  };
}
export const library = catalog as AvatarExpression[];
export const workspaceKey = 'avatar-lab-workspace-v5';
export type ConfigurationScope = Assignment | 'appearance' | 'assignments';
export const configurationPart = (
  setup: AssignedSetup,
  scope: ConfigurationScope,
) =>
  scope === 'appearance' || scope === 'assignments'
    ? setup[scope]
    : setup.assignments[scope];
export function withConfigurationScope<T extends AssignedSetup>(
  target: T,
  source: AssignedSetup,
  scope: ConfigurationScope,
): T {
  return scope === 'appearance' || scope === 'assignments'
    ? { ...target, [scope]: structuredClone(source[scope]) }
    : {
        ...target,
        assignments: {
          ...target.assignments,
          [scope]: structuredClone(source.assignments[scope]),
        },
      };
}
export function makeWorkspace(saved = assignedSetup()): AvatarSetup {
  return {
    ...configuration(saved),
    expressions: structuredClone(library),
    candidates: {},
  };
}
export function configuration(setup: AssignedSetup): AssignedSetup {
  return structuredClone({
    appearance: setup.appearance,
    assignments: setup.assignments,
  });
}
export function readWorkspace(raw: string | null): AvatarSetup {
  if (!raw) return makeWorkspace();
  try {
    const value = JSON.parse(raw);
    // Preserve the last lab draft when upgrading from shared preset references.
    if (value.version === 2 && Array.isArray(value.expressions)) {
      value.assignments = Object.fromEntries(
        assignmentNames.map((a) => [
          a,
          typeof value.assignments[a] === 'string'
            ? structuredClone(
                value.expressions.find(
                  (e: AvatarExpression) => e.id === value.assignments[a],
                ) ?? null,
              )
            : value.assignments[a],
        ]),
      );
    }
    const edits = value.expressions;
    const candidates = value.candidates;
    if (!isAvatarSetup(value)) return makeWorkspace();
    const workspace = makeWorkspace(value);
    workspace.expressions = library.map((e) => {
      const edited =
        Array.isArray(edits) &&
        edits.find((candidate: AvatarExpression) => candidate.id === e.id);
      return isExpression(edited) ? edited : structuredClone(e);
    });
    if (candidates && typeof candidates === 'object') {
      for (const [key, candidate] of Object.entries(candidates)) {
        const c = candidate as Candidate;
        if (
          c &&
          (c.original === null || isExpression(c.original)) &&
          (c.expression === null || isExpression(c.expression))
        )
          workspace.candidates[key] = structuredClone(c);
      }
    }
    return workspace;
  } catch {
    return makeWorkspace();
  }
}
